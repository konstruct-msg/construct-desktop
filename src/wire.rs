use rand::{rngs::OsRng, RngCore};

const MESSAGE_PADDING_MAGIC: [u8; 4] = [0x4B, 0x50, 0x41, 0x44]; // "KPAD"
const MESSAGE_PADDING_HEADER_LEN: usize = 8; // 4 bytes magic + 4 bytes original length (u32 BE)

const MESSAGE_PADDING_BUCKETS: [usize; 3] = [1024, 4096, 16384];
const MESSAGE_PADDING_ENABLED: bool = true;

/// Wire format of `Envelope.encrypted_payload` (classic, no PQC):
/// [0..4)   message_number (u32 LE)
/// [4..36)  dh_public_key (32 bytes)
/// [36..40) one_time_prekey_id (u32 LE)
/// [40..44) kyber_otpk_id (u32 LE; 0 in classic)
/// [44..46) kem_ciphertext_len (u16 LE; 0 in classic)
/// then optional kem_ciphertext bytes
/// then sealed_box bytes (nonce||ciphertext||auth_tag), which correspond to
/// `MessageCryptoService.encrypted.content` base64-decoded payload.
const WIRE_HEADER_LEN: usize = 46;

#[derive(Debug, Clone)]
pub struct DecodedWirePayload {
    pub message_number: u32,
    pub ephemeral_public_key: Vec<u8>, // 32 bytes
    pub one_time_prekey_id: u32,
    pub kyber_otpk_id: u32,
    pub kem_ciphertext: Vec<u8>,
    /// Base64 string for `sealed_box` bytes (padded if padding enabled on sender).
    pub content_padded_base64: String,
}

pub fn encode_encrypted_payload(
    message_number: u32,
    ephemeral_public_key: &[u8],
    one_time_prekey_id: u32,
    kyber_otpk_id: u32,
    kem_ciphertext: Option<&[u8]>,
    // base64 of nonce||ciphertext||auth_tag, already padded (or not - depends on sender policy)
    content_base64: &str,
) -> Result<Vec<u8>, String> {
    if ephemeral_public_key.len() != 32 {
        return Err("ephemeral_public_key must be 32 bytes".into());
    }

    let sealed_box = base64::engine::general_purpose::STANDARD
        .decode(content_base64)
        .map_err(|_| "invalid base64 content".to_string())?;

    let kem = kem_ciphertext.unwrap_or(&[]);
    let kem_len = kem.len();
    if kem_len > u16::MAX as usize {
        return Err(format!(
            "kem_ciphertext too large: {kem_len} bytes"
        ));
    }

    let mut out = Vec::with_capacity(WIRE_HEADER_LEN + kem_len + sealed_box.len());

    out.extend_from_slice(&message_number.to_le_bytes());
    out.extend_from_slice(ephemeral_public_key);
    out.extend_from_slice(&one_time_prekey_id.to_le_bytes());
    out.extend_from_slice(&kyber_otpk_id.to_le_bytes());
    out.extend_from_slice(&(kem_len as u16).to_le_bytes());
    if kem_len > 0 {
        out.extend_from_slice(kem);
    }
    out.extend_from_slice(&sealed_box);

    Ok(out)
}

pub fn decode_encrypted_payload(payload: &[u8]) -> Result<DecodedWirePayload, String> {
    if payload.len() < WIRE_HEADER_LEN + 1 {
        return Err("payload too short".into());
    }

    let message_number = u32::from_le_bytes(payload[0..4].try_into().unwrap());
    let ephemeral_public_key = payload[4..36].to_vec();
    let one_time_prekey_id = u32::from_le_bytes(payload[36..40].try_into().unwrap());
    let kyber_otpk_id = u32::from_le_bytes(payload[40..44].try_into().unwrap());
    let kem_len = u16::from_le_bytes(payload[44..46].try_into().unwrap()) as usize;

    let mut offset = WIRE_HEADER_LEN;
    if payload.len() < offset + kem_len + 1 {
        return Err("payload kem section out of bounds".into());
    }
    let kem_ciphertext = payload[offset..offset + kem_len].to_vec();
    offset += kem_len;

    let sealed_box_bytes = payload[offset..].to_vec();
    let content_padded_base64 = base64::engine::general_purpose::STANDARD
        .encode(sealed_box_bytes);

    Ok(DecodedWirePayload {
        message_number,
        ephemeral_public_key,
        one_time_prekey_id,
        kyber_otpk_id,
        kem_ciphertext,
        content_padded_base64,
    })
}

pub fn pad_ciphertext_base64(base64_in: &str) -> String {
    if !MESSAGE_PADDING_ENABLED {
        return base64_in.to_string();
    }

    let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(base64_in) else {
        return base64_in.to_string();
    };

    // Pick the first bucket where (raw_len + header_len) <= bucket_size.
    let target = MESSAGE_PADDING_BUCKETS
        .iter()
        .copied()
        .find(|b| raw.len() + MESSAGE_PADDING_HEADER_LEN <= *b);

    let Some(target) = target else {
        return base64_in.to_string();
    };

    // If the ciphertext already fits, we still need at least 4 bytes magic + length.
    if target < raw.len() + MESSAGE_PADDING_HEADER_LEN {
        return base64_in.to_string();
    }

    let mut out = Vec::with_capacity(target);
    out.extend_from_slice(&MESSAGE_PADDING_MAGIC);
    let raw_len = raw.len() as u32;
    out.extend_from_slice(&raw_len.to_be_bytes());
    out.extend_from_slice(&raw);

    let padding_len = target - out.len();
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        OsRng.fill_bytes(&mut padding);
        out.extend_from_slice(&padding);
    }

    base64::engine::general_purpose::STANDARD.encode(out)
}

pub fn unpad_ciphertext_base64(base64_in: &str) -> String {
    if !MESSAGE_PADDING_ENABLED {
        return base64_in.to_string();
    }

    let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(base64_in) else {
        return base64_in.to_string();
    };
    if raw.len() < MESSAGE_PADDING_HEADER_LEN {
        return base64_in.to_string();
    }

    if raw[0..4] != MESSAGE_PADDING_MAGIC {
        return base64_in.to_string();
    }

    let original_len = u32::from_be_bytes(raw[4..8].try_into().unwrap()) as usize;
    if original_len == 0 || original_len > raw.len() - MESSAGE_PADDING_HEADER_LEN {
        return base64_in.to_string();
    }

    let start = MESSAGE_PADDING_HEADER_LEN;
    let end = start + original_len;
    let trimmed = &raw[start..end];
    base64::engine::general_purpose::STANDARD.encode(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_then_unpad_roundtrip() {
        let raw = vec![7u8; 100];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
        let padded = pad_ciphertext_base64(&b64);
        let unpadded = unpad_ciphertext_base64(&padded);
        assert_eq!(unpadded, b64);
    }

    #[test]
    fn encode_decode_roundtrip_classic() {
        let ephemeral = vec![1u8; 32];
        let sealed = vec![2u8; 64]; // includes nonce+ciphertext bytes; padding handled above
        let content_b64 = base64::engine::general_purpose::STANDARD.encode(&sealed);

        let payload = encode_encrypted_payload(
            0,
            &ephemeral,
            123,
            0,
            None,
            &content_b64,
        )
        .unwrap();

        let decoded = decode_encrypted_payload(&payload).unwrap();
        assert_eq!(decoded.message_number, 0);
        assert_eq!(decoded.ephemeral_public_key, ephemeral);
        assert_eq!(decoded.one_time_prekey_id, 123);
        assert_eq!(decoded.kyber_otpk_id, 0);

        let sealed_decoded = base64::engine::general_purpose::STANDARD.decode(&decoded.content_padded_base64).unwrap();
        assert_eq!(sealed_decoded, sealed);
    }
}

