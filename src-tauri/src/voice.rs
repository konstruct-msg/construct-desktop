//! Voice notes in the window: recording one (construct-client's `voice`, the microphone and the
//! AAC encoder) and playing one.
//!
//! A note from a phone is AAC in an MP4 file. WebKitGTK plays AAC only with GStreamer's libav or
//! fdk plugins, which a stock Ubuntu or Debian lacks, so the page is given the sound decoded here,
//! as WAV: the same on every system. Symphonia decodes it (pure Rust).

use std::sync::Mutex;
use std::time::Duration;

use construct_client::voice::Recorder;
use tauri::{AppHandle, Manager};

use crate::Intents;
use crate::bridge::{Intent, ViewEvent};

/// How often the page hears the length and level of a recording under way.
const TICK: Duration = Duration::from_millis(100);

/// The recording under way, for whom, and which one (a stop and a new start in between would
/// otherwise let an old ticker end the new one).
#[derive(Default)]
pub struct Recording(Mutex<Option<Held>>);

struct Held {
    recorder: Recorder,
    peer_id: String,
    id: u64,
}

fn show(app: &AppHandle, event: ViewEvent) {
    app.state::<Intents>().send(Intent::Recording(event));
}

fn state(peer_id: &str, elapsed_ms: u32, level: f32, recording: bool) -> ViewEvent {
    ViewEvent::Recording {
        peer_id: peer_id.to_owned(),
        recording,
        elapsed_ms,
        level,
    }
}

/// Start recording a note for `peer_id`; one at a time.
pub fn start(app: &AppHandle, peer_id: String) -> Result<(), String> {
    let held = app.state::<Recording>();
    let mut held = held.0.lock().unwrap_or_else(|e| e.into_inner());
    if held.is_some() {
        return Err("already recording".into());
    }
    let recorder = Recorder::start()?;
    let id = next_id();
    tracing::info!("voice note: recording");
    *held = Some(Held {
        recorder,
        peer_id: peer_id.clone(),
        id,
    });
    show(app, state(&peer_id, 0, 0.0, true));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            let full = {
                let held = app.state::<Recording>();
                let held = held.0.lock().unwrap_or_else(|e| e.into_inner());
                let Some(h) = held.as_ref().filter(|h| h.id == id) else {
                    return;
                };
                show(
                    &app,
                    state(
                        &h.peer_id,
                        h.recorder.elapsed_ms(),
                        h.recorder.level(),
                        true,
                    ),
                );
                h.recorder.full()
            };
            // Five minutes: sent as it is, as the phones stop there.
            if full {
                let _ = stop(&app, true).await;
                return;
            }
        }
    });
    Ok(())
}

/// End the recording: sent when `send`, else dropped.
pub async fn stop(app: &AppHandle, send: bool) -> Result<(), String> {
    let held = app
        .state::<Recording>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or("not recording")?;
    show(app, state(&held.peer_id, 0, 0.0, false));
    if !send {
        tracing::info!("voice note: dropped");
        return Ok(());
    }
    let peer_id = held.peer_id;
    let voice = tauri::async_runtime::spawn_blocking(move || held.recorder.finish())
        .await
        .map_err(|e| e.to_string())??;
    tracing::info!(
        ms = voice.duration_ms,
        bytes = voice.bytes.len(),
        "voice note: recorded"
    );
    app.state::<Intents>()
        .send(Intent::SendVoice { peer_id, voice });
    Ok(())
}

fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// The sound of an MP4 file as a 16-bit WAV, no larger than `max` bytes: at its own rate and
/// channels when that fits, else mixed to mono and halved in rate until it does. `None` when it
/// is not AAC in MP4 that decodes.
pub fn wav_of_mp4(bytes: &[u8], max: usize) -> Option<Vec<u8>> {
    let (mut pcm, mut channels, mut rate) = decode(bytes)?;
    let wav_len = |samples: usize| 44 + samples * 2;
    if wav_len(pcm.len()) > max && channels > 1 {
        pcm = pcm
            .chunks(channels)
            .map(|f| f.iter().sum::<f32>() / f.len() as f32)
            .collect();
        channels = 1;
    }
    while wav_len(pcm.len()) > max && rate > 8_000 {
        pcm = pcm
            .chunks(2)
            .map(|p| p.iter().sum::<f32>() / p.len() as f32)
            .collect();
        rate /= 2;
    }
    (wav_len(pcm.len()) <= max).then(|| wav(&pcm, channels as u16, rate))
}

fn decode(bytes: &[u8]) -> Option<(Vec<f32>, usize, u32)> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let source = MediaSourceStream::new(
        Box::new(std::io::Cursor::new(bytes.to_vec())),
        Default::default(),
    );
    let mut hint = Hint::new();
    hint.with_extension("m4a");
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .ok()?;
    let track = format.default_track(TrackType::Audio)?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(
            track.codec_params.as_ref()?.audio()?,
            &AudioDecoderOptions::default(),
        )
        .ok()?;
    let (mut pcm, mut channels, mut rate) = (Vec::new(), 0, 0);
    let mut buf = Vec::new();
    while let Ok(Some(packet)) = format.next_packet() {
        if packet.track_id != track_id {
            continue;
        }
        let Ok(decoded) = decoder.decode(&packet) else {
            continue;
        };
        channels = decoded.spec().channels().count();
        rate = decoded.spec().rate();
        buf.resize(decoded.samples_interleaved(), 0.0f32);
        decoded.copy_to_slice_interleaved(&mut buf);
        pcm.extend_from_slice(&buf);
    }
    (channels > 0 && rate > 0 && !pcm.is_empty()).then_some((pcm, channels, rate))
}

fn wav(pcm: &[f32], channels: u16, rate: u32) -> Vec<u8> {
    let data = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(secs: f32, rate: u32) -> Vec<u8> {
        let pcm: Vec<f32> = (0..(secs * rate as f32) as usize)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 0.5)
            .collect();
        construct_client::voice::encode(&pcm, rate).unwrap().bytes
    }

    fn header(wav: &[u8]) -> (u16, u32, usize) {
        let channels = u16::from_le_bytes([wav[22], wav[23]]);
        let rate = u32::from_le_bytes(wav[24..28].try_into().unwrap());
        let samples = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize / 2;
        (channels, rate, samples)
    }

    /// A note as the phones send it plays from WAV: its rate, its length, its sound.
    #[test]
    fn a_voice_note_becomes_a_wav_of_its_length() {
        let wav = wav_of_mp4(&note(2.0, 44_100), 25 << 20).unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        let (channels, rate, samples) = header(&wav);
        assert_eq!((channels, rate), (1, 44_100));
        // Two seconds, and the encoder's padding at the ends.
        let secs = samples as f32 / rate as f32;
        assert!((2.0..2.2).contains(&secs), "{secs} s");
        let loudest = wav[44..]
            .chunks(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]).unsigned_abs())
            .max()
            .unwrap();
        assert!(loudest > 10_000, "the tone is in it: {loudest}");
    }

    /// Too large for the page at its own rate: halved until it fits.
    #[test]
    fn a_long_note_is_made_to_fit() {
        let full = wav_of_mp4(&note(2.0, 48_000), 25 << 20).unwrap();
        let small = wav_of_mp4(&note(2.0, 48_000), full.len() / 2 + 44).unwrap();
        assert!(small.len() <= full.len() / 2 + 44);
        assert_eq!(header(&small).1, 24_000);
    }

    #[test]
    fn what_is_not_aac_in_mp4_is_not_decoded() {
        assert!(wav_of_mp4(b"\0\0\0\x18ftypM4A not really", 1 << 20).is_none());
        assert!(wav_of_mp4(b"OggS....", 1 << 20).is_none());
    }
}
