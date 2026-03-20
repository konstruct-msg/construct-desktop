use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use construct_core::crypto::client_api::ClassicClient;
use construct_core::crypto::suites::classic::ClassicSuiteProvider;
use construct_core::orchestration::{Action, Orchestrator};

use crate::storage::SecureStore;
use crate::grpc::{AuthServiceApi, GrpcConfig, GrpcTransport, KeyServiceApi};
use crate::grpc::services;
use base64::Engine as _;
use construct_core::device_id::derive_device_id;
use construct_core::pow::compute_pow;
use crate::wire::{decode_encrypted_payload, unpad_ciphertext_base64, pad_ciphertext_base64, encode_encrypted_payload};

#[derive(Debug, Clone)]
pub struct ChatSummary {
    pub chat_id: String,
    pub title: String,
    pub last_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub chat_id: String,
    pub from: String,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub enum CoreEvent {
    /// Successfully decrypted plaintext.
    MessageDecrypted {
        contact_id: String,
        message_id: String,
        plaintext: String,
    },
    NotifyError {
        code: String,
        message: String,
    },
    NotifySessionCreated {
        contact_id: String,
    },
    /// Fallback when we only have a preview from routing.
    NotifyNewMessage {
        chat_id: String,
        preview: String,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SendTextResult {
    pub message_id: String,
}

#[derive(Debug, Default)]
pub struct EngineState {
    pub is_authenticated: bool,
    pub user_id: Option<String>,
    pub chats: Vec<ChatSummary>,
    pub messages: HashMap<String, Vec<ChatMessage>>,
    pub stream_connected: bool,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub device_id: Option<String>,
    pub receiving_loop_running: bool,
}

/// Main application engine (auth + crypto + messaging).
///
/// For MVP we start with a placeholder state. Subsequent to-dos will wire
/// in `construct-core` and gRPC streaming.
pub struct ConstructEngine {
    pub(crate) state: Arc<Mutex<EngineState>>,
    #[allow(dead_code)]
    pub(crate) secure_store: SecureStore,
    core: Arc<Mutex<Option<Orchestrator>>>,
}

impl ConstructEngine {
    pub async fn new_default() -> Self {
        let secure_store = SecureStore::new_default().await;
        let core = restore_orchestrator_from_store(&secure_store).await.ok();
        let user_id = if core.is_some() {
            load_user_id(&secure_store).ok().flatten()
        } else {
            None
        };

        Self {
            state: Arc::new(Mutex::new(EngineState {
                is_authenticated: core.is_some(),
                user_id,
                ..Default::default()
            })),
            secure_store,
            core: Arc::new(Mutex::new(core)),
        }
    }

    /// Start MessagingService stream after auth/core is ready.
    ///
    /// MVP placeholder: will be implemented in `receive-loop` to-do.
    pub async fn start_stream(&self) -> anyhow::Result<()> {
        let mut st = self.state.lock().await;
        st.stream_connected = true;
        Ok(())
    }

    pub async fn save_auth_tokens(
        &self,
        user_id: String,
        device_id: String,
        access_token: String,
        refresh_token: String,
    ) -> anyhow::Result<()> {
        {
            let mut st = self.state.lock().await;
            st.is_authenticated = true;
            st.user_id = Some(user_id.clone());
            st.device_id = Some(device_id.clone());
            st.access_token = Some(access_token.clone());
            st.refresh_token = Some(refresh_token.clone());
        }

        self.secure_store.put_bytes("user_id", user_id.as_bytes())?;
        self.secure_store.put_bytes("device_id", device_id.as_bytes())?;
        self.secure_store.put_bytes("access_token", access_token.as_bytes())?;
        self.secure_store.put_bytes("refresh_token", refresh_token.as_bytes())?;
        Ok(())
    }

    pub async fn load_auth_from_store(&self) -> anyhow::Result<()> {
        let user_id = self
            .secure_store
            .get_bytes("user_id")?
            .and_then(|b| String::from_utf8(b).ok());
        let device_id = self
            .secure_store
            .get_bytes("device_id")?
            .and_then(|b| String::from_utf8(b).ok());
        let access_token = self
            .secure_store
            .get_bytes("access_token")?
            .and_then(|b| String::from_utf8(b).ok());
        let refresh_token = self
            .secure_store
            .get_bytes("refresh_token")?
            .and_then(|b| String::from_utf8(b).ok());

        let mut st = self.state.lock().await;
        st.user_id = user_id;
        st.device_id = device_id;
        st.access_token = access_token;
        st.refresh_token = refresh_token;
        st.is_authenticated = st.user_id.is_some() && st.access_token.is_some();
        Ok(())
    }

    /// Device-based registration/login (MVP).
    ///
    /// 1) If `private_keys_json` is missing: generates a new device core and performs
    ///    PoW + `AuthService.RegisterDevice`.
    /// 2) If keys exist: signs `KonstruktAuth-v1\\n{device_id}\\n{timestamp}` and calls
    ///    `AuthService.AuthenticateDevice`.
    ///
    /// On success persists:
    /// - tokens + `user_id` + `device_id`
    /// - `otpks_json = []` and initial `orchestrator_state` so core can be restored
    pub async fn ensure_device_auth(&self, username: Option<String>) -> anyhow::Result<()> {
        let is_new_device = self
            .secure_store
            .get_bytes("private_keys_json")?
            .is_none();

        let private_keys_json = match self.secure_store.get_bytes("private_keys_json")? {
            Some(b) => String::from_utf8(b)?,
            None => {
                let client = ClassicClient::<ClassicSuiteProvider>::new()
                    .map_err(|e| anyhow::anyhow!(e))?;
                let orch = Orchestrator::new(client, "temp_user".to_string());
                let pk_json = orch
                    .export_private_keys_json_str()
                    .map_err(|e| anyhow::anyhow!(e))?;
                self.secure_store.put_bytes("private_keys_json", pk_json.as_bytes())?;
                pk_json
            }
        };

        let keys: PrivateKeysJson = serde_json::from_str(&private_keys_json)?;

        let identity_secret =
            base64::engine::general_purpose::STANDARD.decode(keys.identity_secret)?;
        let signing_secret =
            base64::engine::general_purpose::STANDARD.decode(keys.signing_secret)?;
        let prekey_secret = base64::engine::general_purpose::STANDARD
            .decode(keys.signed_prekey_secret)?;
        let prekey_signature = base64::engine::general_purpose::STANDARD
            .decode(keys.prekey_signature)?;

        let client = ClassicClient::<ClassicSuiteProvider>::from_keys(
            identity_secret,
            signing_secret,
            prekey_secret,
            prekey_signature,
        ).map_err(|e| anyhow::anyhow!(e))?;

        let orch = Orchestrator::new(client, "temp_user".to_string());

        // Derive device_id from identity_public.
        let device_bundle_json = orch
            .export_registration_bundle_json_str()
            .map_err(|e| anyhow::anyhow!(e))?;
        let device_bundle: RegistrationBundleJson = serde_json::from_str(&device_bundle_json)?;
        let identity_public_bytes =
            base64::engine::general_purpose::STANDARD.decode(device_bundle.identity_public)?;
        let device_id = derive_device_id(&identity_public_bytes);

        self.secure_store.put_bytes("device_id", device_id.as_bytes())?;

        // gRPC auth.
        let transport = GrpcTransport::connect(GrpcConfig::default()).await?;
        let mut auth_api = AuthServiceApi::new(transport.channel());

        let (user_id, access_token, refresh_token) = if is_new_device {
            let pow_chal = auth_api.get_pow_challenge().await?;
            let pow_solution = compute_pow(&pow_chal.challenge, pow_chal.difficulty);

            let pow_solution_proto = services::PowSolution {
                challenge: pow_chal.challenge,
                nonce: pow_solution.nonce,
                hash: pow_solution.hash,
            };

            let public_keys = services::DevicePublicKeys {
                verifying_key: device_bundle.verifying_key,
                identity_public: device_bundle.identity_public,
                signed_prekey_public: device_bundle.signed_prekey_public,
                signed_prekey_signature: device_bundle.signature,
                crypto_suite: "Curve25519+Ed25519".to_string(),
            };

            let req = services::RegisterDeviceRequest {
                username,
                device_id: device_id.clone(),
                public_keys,
                pow_solution: pow_solution_proto,
            };

            let resp = auth_api.register_device(req).await?;
            (resp.user_id, resp.access_token, resp.refresh_token)
        } else {
            let timestamp = current_timestamp_seconds();
            let msg = format!("KonstruktAuth-v1\\n{}\\n{}", device_id, timestamp);
            let signature_b64 = orch
                .sign_bundle_bytes(msg.as_bytes())
                .map_err(|e| anyhow::anyhow!(e))?;

            let req = services::AuthenticateDeviceRequest {
                device_id: device_id.clone(),
                timestamp,
                signature: signature_b64,
            };

            let resp = auth_api.authenticate_device(req).await?;
            (resp.user_id, resp.access_token, resp.refresh_token)
        };

        // Persist initial core state for later decrypt/send.
        let client2 = ClassicClient::<ClassicSuiteProvider>::from_keys(
            base64::engine::general_purpose::STANDARD.decode(keys.identity_secret)?,
            base64::engine::general_purpose::STANDARD.decode(keys.signing_secret)?,
            base64::engine::general_purpose::STANDARD.decode(keys.signed_prekey_secret)?,
            base64::engine::general_purpose::STANDARD.decode(keys.prekey_signature)?,
        ).map_err(|e| anyhow::anyhow!(e))?;

        let mut new_orch = Orchestrator::new(client2, user_id.clone());
        new_orch
            .import_otpks_json("[]")
            .map_err(|e| anyhow::anyhow!(e))?;
        let orchestrator_state = new_orch
            .export_orchestrator_state_cfe()
            .map_err(|e| anyhow::anyhow!(e))?;
        self.secure_store.put_bytes("otpks_json", b"[]")?;
        self.secure_store.put_bytes("orchestrator_state", &orchestrator_state)?;

        let device_id_for_prekeys = device_id.clone();
        let access_token_for_prekeys = access_token.clone();

        self.save_auth_tokens(
            user_id.clone(),
            device_id,
            access_token,
            refresh_token,
        )
        .await?;

        // Swap core in-memory.
        let mut core_guard = self.core.lock().await;
        *core_guard = Some(new_orch);
        drop(core_guard);

        // Generate/upload initial OTPKs if needed (MVP threshold).
        self.generate_and_upload_otpks_if_needed(50, &device_id_for_prekeys, &access_token_for_prekeys)
            .await?;

        Ok(())
    }

    async fn generate_and_upload_otpks_if_needed(
        &self,
        initial_count: u32,
        device_id: &str,
        access_token: &str,
    ) -> anyhow::Result<()> {
        // Connect to gRPC and fetch server-side OTPK count.
        let transport = GrpcTransport::connect(GrpcConfig::default()).await?;
        let mut key_api = KeyServiceApi::new(transport.channel());
        let metadata = transport.auth_metadata(access_token)?;

        let current = key_api
            .get_pre_key_count(
                services::GetPreKeyCountRequest {
                    device_id: device_id.to_string(),
                },
                Some(metadata),
            )
            .await?;

        let recommended = current.recommended_minimum;
        let server_count = current.count;

        if server_count >= recommended {
            return Ok(());
        }

        let mut needed = recommended.saturating_sub(server_count);
        needed = needed.max(initial_count);
        if needed == 0 {
            return Ok(());
        }

        // Generate OTPKs in core and upload public keys.
        let mut orch = self.core.lock().await;
        let Some(core) = orch.as_mut() else {
            return Err(anyhow::anyhow!("core not initialized"));
        };

        let generated = core
            .generate_otpks(needed)
            .map_err(|e| anyhow::anyhow!(e))?;

        let pre_keys: Vec<services::OneTimePreKey> = generated
            .into_iter()
            .map(|(key_id, public_key)| services::OneTimePreKey {
                key_id,
                public_key,
            })
            .collect();

        let upload_req = services::UploadPreKeysRequest {
            device_id: device_id.to_string(),
            pre_keys,
            signed_pre_key: None,
            replace_existing: false,
            kyber_pre_keys: vec![],
            kyber_signed_pre_key: None,
        };

        let _upload_resp = key_api
            .upload_pre_keys(upload_req, Some(transport.auth_metadata(access_token)?))
            .await?;

        // Persist updated OTPKs + orchestrator state to secure storage.
        let otpks_json = core.export_otpks_json().map_err(|e| anyhow::anyhow!(e))?;
        let state = core
            .export_orchestrator_state_cfe()
            .map_err(|e| anyhow::anyhow!(e))?;

        self.secure_store.put_bytes("otpks_json", otpks_json.as_bytes())?;
        self.secure_store.put_bytes("orchestrator_state", &state)?;

        Ok(())
    }

    /// Long-lived receiver loop (MVP polling).
    ///
    /// Note: to keep MVP simple and robust, this uses `GetPendingMessages`
    /// polling instead of `MessageStream`. It still performs the required
    /// session-init for first messages (`message_number == 0`).
    pub async fn start_receive_loop(&self) -> anyhow::Result<()> {
        let (access_token, device_id) = {
            let st = self.state.lock().await;
            (
                st.access_token.clone().ok_or_else(|| anyhow::anyhow!("missing access_token"))?,
                st.device_id.clone().ok_or_else(|| anyhow::anyhow!("missing device_id"))?,
            )
        };

        let should_spawn = {
            let mut st = self.state.lock().await;
            if st.receiving_loop_running {
                false
            } else {
                st.receiving_loop_running = true;
                true
            }
        };

        if !should_spawn {
            return Ok(());
        }

        let state_arc = self.state.clone();
        let core_arc = self.core.clone();
        let secure_store = self.secure_store.clone();
        let access_token_clone = access_token.clone();

        tokio::spawn(async move {
            let transport = match GrpcTransport::connect(GrpcConfig::default()).await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("receive_loop: transport connect failed: {e}");
                    return;
                }
            };

            let metadata = match transport.auth_metadata(&access_token_clone) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("receive_loop: auth_metadata failed: {e}");
                    return;
                }
            };

            let mut key_api = KeyServiceApi::new(transport.channel());
            let mut messaging_api = crate::grpc::MessagingApi::new(transport.channel());

            let mut cursor: Option<String> = match secure_store.get_bytes("pending_cursor") {
                Ok(Some(bytes)) => String::from_utf8(bytes).ok(),
                _ => None,
            };

            loop {
                let req = services::GetPendingMessagesRequest {
                    since_cursor: cursor.clone(),
                    limit: Some(50),
                };

                let resp = match messaging_api
                    .get_pending_messages(req, Some(metadata.clone()))
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("receive_loop: GetPendingMessages failed: {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                };

                cursor = Some(resp.next_cursor.clone());
                let _ = secure_store.put_bytes("pending_cursor", resp.next_cursor.as_bytes());

                for pending in resp.messages {
                    // Try decoding wire payload; if it fails (e.g., control messages), skip for MVP.
                    let Ok(decoded) = decode_encrypted_payload(&pending.encrypted_payload) else {
                        continue;
                    };

                    let contact_id = pending.sender_id.clone();
                    let msg_num = decoded.message_number;
                    let is_first_message = msg_num == 0;

                    // Decode sealed box content for both init and decrypt.
                    let unpadded_content_b64 = unpad_ciphertext_base64(&decoded.content_padded_base64);
                    let sealed_box = match base64::engine::general_purpose::STANDARD
                        .decode(&unpadded_content_b64)
                    {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    if sealed_box.len() < 12 {
                        continue;
                    }
                    let nonce = sealed_box[..12].to_vec();
                    let ciphertext = sealed_box[12..].to_vec();

                    // Session-init for first message when no active session exists.
                    if is_first_message {
                        let need_init = {
                            let mut core_guard = core_arc.lock().await;
                            let Some(core) = core_guard.as_ref() else { false };
                            !core.has_active_session(&contact_id)
                        };

                        if need_init {
                            // Fetch sender public pre-key bundle.
                            let get_bundle = services::GetPreKeyBundleRequest {
                                user_id: contact_id.clone(),
                                device_id: None,
                                preferred_suite: None,
                            };

                            let bundle_resp = match key_api
                                .get_pre_key_bundle(get_bundle, Some(metadata.clone()))
                                .await
                            {
                                Ok(b) => b,
                                Err(_) => continue,
                            };

                            let suite_id = suite_id_from_crypto_suite(&bundle_resp.bundle.crypto_suite);

                            let recipient_bundle = serde_json::json!({
                                "identity_public": bundle_resp.bundle.identity_key,
                                "signed_prekey_public": bundle_resp.bundle.signed_pre_key,
                                "signature": bundle_resp.bundle.signed_pre_key_signature,
                                "verifying_key": bundle_resp.verifying_key,
                                "suite_id": suite_id
                            });
                            let recipient_bundle_bytes =
                                serde_json::to_vec(&recipient_bundle).unwrap_or_default();

                            let first_message = serde_json::json!({
                                "ephemeral_public_key": decoded.ephemeral_public_key.clone(),
                                "message_number": decoded.message_number,
                                "content": unpadded_content_b64,
                                "one_time_prekey_id": decoded.one_time_prekey_id
                            });
                            let first_message_bytes =
                                serde_json::to_vec(&first_message).unwrap_or_default();

                            // Initialize receiving session in core.
                            let mut core_guard = core_arc.lock().await;
                            let Some(core) = core_guard.as_mut() else { continue };
                            let _ = core.init_receiving_session_with_msg(
                                &contact_id,
                                &recipient_bundle_bytes,
                                &first_message_bytes,
                            );
                        }
                    }

                    // Build wire_json consumed by `construct-core` decrypt path.
                    let wire_message = serde_json::json!({
                        "dh_public_key": decoded.ephemeral_public_key.clone(),
                        "message_number": decoded.message_number,
                        "ciphertext": ciphertext,
                        "nonce": nonce,
                        "previous_chain_length": 0,
                        "suite_id": 1
                    });
                    let wire_json_bytes = match serde_json::to_vec(&wire_message) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };

                    // Decrypt via orchestrator.
                    let actions = {
                        let mut core_guard = core_arc.lock().await;
                        let Some(core) = core_guard.as_mut() else { continue };
                        core.handle_event(construct_core::orchestration::IncomingEvent::MessageReceived {
                            message_id: pending.message_id.clone(),
                            from: contact_id.clone(),
                            data: wire_json_bytes.clone(),
                            msg_num,
                            kem_ct: decoded.kem_ciphertext,
                            otpk_id: decoded.one_time_prekey_id,
                            is_control: false,
                        })
                    };

                    // Persist state + collect UI-visible events.
                    let core_events = {
                        let mut core_guard = core_arc.lock().await;
                        let Some(core) = core_guard.as_mut() else { vec![] };

                        let mut events = Vec::new();
                        for action in actions {
                            match action {
                                construct_core::orchestration::Action::SaveSessionToSecureStore { key, data } => {
                                    if data.is_empty() {
                                        let _ = secure_store.delete(&key);
                                    } else {
                                        let _ = secure_store.put_bytes(&key, &data);
                                    }
                                }
                                construct_core::orchestration::Action::MessageDecrypted { contact_id, message_id, plaintext_utf8 } => {
                                    events.push(CoreEvent::MessageDecrypted { contact_id, message_id, plaintext: plaintext_utf8 });
                                }
                                construct_core::orchestration::Action::NotifyError { code, message } => {
                                    events.push(CoreEvent::NotifyError { code, message });
                                }
                                construct_core::orchestration::Action::NotifyNewMessage { chat_id, preview } => {
                                    events.push(CoreEvent::NotifyNewMessage { chat_id, preview });
                                }
                                construct_core::orchestration::Action::NotifySessionCreated { contact_id } => {
                                    events.push(CoreEvent::NotifySessionCreated { contact_id });
                                }
                                _ => {}
                            }
                        }

                        if let Ok(state_bytes) = core.export_orchestrator_state_cfe() {
                            let _ = secure_store.put_bytes("orchestrator_state", &state_bytes);
                        }
                        events
                    };

                    // Apply events to in-memory UI state.
                    if !core_events.is_empty() {
                        let mut st = state_arc.lock().await;
                        for ev in core_events {
                            match ev {
                                CoreEvent::MessageDecrypted { contact_id, message_id, plaintext } => {
                                    let chat_id = contact_id.clone();
                                    let list = st.messages.entry(chat_id.clone()).or_default();
                                    if !list.iter().any(|m| m.id == message_id) {
                                        let now = current_millis();
                                        list.push(ChatMessage {
                                            id: message_id.clone(),
                                            chat_id: chat_id.clone(),
                                            from: contact_id.clone(),
                                            text: plaintext,
                                            timestamp_ms: now,
                                        });
                                    }

                                    if let Some(chat) = st.chats.iter_mut().find(|c| c.chat_id == chat_id) {
                                        chat.last_message = Some(
                                            list.last().map(|m| m.text.clone()).unwrap_or_default()
                                        );
                                    } else {
                                        st.chats.push(ChatSummary {
                                            chat_id: chat_id.clone(),
                                            title: chat_id.clone(),
                                            last_message: Some(list.last().map(|m| m.text.clone()).unwrap_or_default()),
                                        });
                                    }
                                }
                                CoreEvent::NotifyNewMessage { chat_id, preview } => {
                                    if let Some(chat) = st.chats.iter_mut().find(|c| c.chat_id == chat_id) {
                                        chat.last_message = Some(preview);
                                    } else {
                                        st.chats.push(ChatSummary { chat_id, title: "chat".to_string(), last_message: Some(preview) });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });

        Ok(())
    }

    /// Execute actions returned by `construct-core` and persist updated state.
    ///
    /// This is the platform responsibility for `Action::SaveSessionToSecureStore`
    /// and for persisting crash-safe `orchestrator_state` blobs.
    pub async fn execute_core_actions(
        &self,
        actions: Vec<Action>,
    ) -> anyhow::Result<Vec<CoreEvent>> {
        let mut orch_guard = self.core.lock().await;
        let Some(orch) = orch_guard.as_mut() else {
            // Nothing to persist without a core.
            return Ok(vec![]);
        };

        let mut events: Vec<CoreEvent> = Vec::new();
        for action in actions {
            match action {
                Action::SaveSessionToSecureStore { key, data } => {
                    // Empty data is a delete sentinel (Rust archive_session delete hot session).
                    if data.is_empty() {
                        let _ = self.secure_store.delete(&key);
                    } else {
                        self.secure_store.put_bytes(&key, &data)?;
                    }
                }
                Action::MessageDecrypted {
                    contact_id,
                    message_id,
                    plaintext_utf8,
                } => {
                    events.push(CoreEvent::MessageDecrypted {
                        contact_id,
                        message_id,
                        plaintext: plaintext_utf8,
                    });
                }
                Action::NotifyNewMessage { chat_id, preview } => {
                    events.push(CoreEvent::NotifyNewMessage { chat_id, preview });
                }
                Action::NotifyError { code, message } => {
                    events.push(CoreEvent::NotifyError { code, message });
                }
                Action::NotifySessionCreated { contact_id } => {
                    events.push(CoreEvent::NotifySessionCreated { contact_id });
                }
                // Other actions will be handled in follow-up to-dos.
                _ => {}
            }
        }

        // Always persist orchestrator coordination state after any significant update.
        let state = orch.export_orchestrator_state_cfe().map_err(|e| anyhow::anyhow!(e))?;
        self.secure_store.put_bytes("orchestrator_state", &state)?;
        Ok(events)
    }

    /// Ensure active session exists for a given contact (used for sending).
    pub async fn ensure_active_session(&self, contact_id: &str) -> anyhow::Result<()> {
        let mut orch = self.core.lock().await;
        let Some(core) = orch.as_mut() else {
            return Err(anyhow::anyhow!("core not initialized"));
        };

        if core.has_active_session(contact_id) {
            return Ok(());
        }

        // Load from secure store archive_<contact_id> if present.
        let archive_key = format!("archive_{contact_id}");
        if let Some(bytes) = self.secure_store.get_bytes(&archive_key)? {
            let json = String::from_utf8(bytes)?;
            core.import_session_json(contact_id, &json)
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        Ok(())
    }

    pub async fn send_text(&self, chat_id: String, text: String) -> anyhow::Result<SendTextResult> {
        // MVP: perform E2EE encryption + wire encoding.
        // Actual gRPC `SendMessage` wiring is a follow-up to keep the MVP moving.
        let access_token = {
            let st = self.state.lock().await;
            st.access_token.clone().ok_or_else(|| anyhow::anyhow!("missing access_token"))?
        };

        // Ensure we have a core to encrypt with.
        let mut need_init = false;
        {
            let mut core_guard = self.core.lock().await;
            let Some(core) = core_guard.as_ref() else {
                return Err(anyhow::anyhow!("core not initialized"));
            };
            need_init = !core.has_active_session(&chat_id);
        }

        if need_init {
            // Fetch recipient pre-key bundle to create the initiator session.
            let transport = GrpcTransport::connect(GrpcConfig::default()).await?;
            let mut key_api = KeyServiceApi::new(transport.channel());
            let metadata = transport.auth_metadata(&access_token)?;

            let bundle_resp = key_api
                .get_pre_key_bundle(
                    services::GetPreKeyBundleRequest {
                        user_id: chat_id.clone(),
                        device_id: None,
                        preferred_suite: None,
                    },
                    Some(metadata),
                )
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            let suite_id = suite_id_from_crypto_suite(&bundle_resp.bundle.crypto_suite);

            let recipient_bundle = serde_json::json!({
                "identity_public": bundle_resp.bundle.identity_key,
                "signed_prekey_public": bundle_resp.bundle.signed_pre_key,
                "signature": bundle_resp.bundle.signed_pre_key_signature,
                "verifying_key": bundle_resp.verifying_key,
                "suite_id": suite_id,
                "one_time_prekey_public": bundle_resp.bundle.one_time_pre_key,
                "one_time_prekey_id": bundle_resp.bundle.one_time_pre_key_id
            });

            let recipient_bundle_bytes = serde_json::to_vec(&recipient_bundle)?;

            let mut core_guard = self.core.lock().await;
            let Some(core) = core_guard.as_mut() else {
                return Err(anyhow::anyhow!("core not initialized"));
            };
            core.init_session_with_bundle(&chat_id, &recipient_bundle_bytes)
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        // Encrypt with an active session.
        let (ephemeral_public_key, message_number, content_b64, one_time_prekey_id) = {
            let mut core_guard = self.core.lock().await;
            let Some(core) = core_guard.as_mut() else {
                return Err(anyhow::anyhow!("core not initialized"));
            };
            core.encrypt_message_for(&chat_id, &text)
                .map_err(|e| anyhow::anyhow!(e))?
        };

        let padded_content_b64 = pad_ciphertext_base64(&content_b64);
        let _encrypted_payload = encode_encrypted_payload(
            message_number,
            &ephemeral_public_key,
            one_time_prekey_id,
            0,
            None,
            &padded_content_b64,
        )
        .map_err(|e| anyhow::anyhow!(e))?;

        // For MVP UI we still store plaintext bubble locally.
        let now = current_millis();
        let msg_id = format!("local-{}", now);

        let mut st = self.state.lock().await;
        st.messages
            .entry(chat_id.clone())
            .or_default()
            .push(ChatMessage {
                id: msg_id.clone(),
                chat_id: chat_id.clone(),
                from: st.user_id.clone().unwrap_or_else(|| "me".to_string()),
                text: text.clone(),
                timestamp_ms: now,
            });

        if let Some(chat) = st.chats.iter_mut().find(|c| c.chat_id == chat_id) {
            chat.last_message = Some(text);
        } else {
            st.chats.push(ChatSummary {
                chat_id: chat_id.clone(),
                title: chat_id.clone(),
                last_message: Some(text),
            });
        }

        Ok(SendTextResult { message_id: msg_id })
    }
}

fn current_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn current_timestamp_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn suite_id_from_crypto_suite(crypto_suite: &str) -> u16 {
    match crypto_suite {
        "X25519_CHACHA20" | "Curve25519+ChaCha20" => 1,
        "X25519_AES256" | "Curve25519+AES256" => 2,
        "KYBER_HYBRID" => 3,
        other => other.parse::<u16>().unwrap_or(1),
    }
}

#[derive(Debug, serde::Deserialize)]
struct PrivateKeysJson {
    identity_secret: String,
    signing_secret: String,
    signed_prekey_secret: String,
    prekey_signature: String,
    #[serde(default)]
    suite_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct RegistrationBundleJson {
    identity_public: String,
    signed_prekey_public: String,
    signature: String,
    verifying_key: String,
    #[serde(default)]
    suite_id: String,
}

async fn restore_orchestrator_from_store(store: &SecureStore) -> anyhow::Result<Orchestrator> {
    let user_id = load_user_id(store)?.ok_or_else(|| anyhow::anyhow!("missing user_id"))?;
    let private_keys_json_bytes = store
        .get_bytes("private_keys_json")?
        .ok_or_else(|| anyhow::anyhow!("missing private_keys_json"))?;
    let otpks_json_bytes = store
        .get_bytes("otpks_json")?
        .ok_or_else(|| anyhow::anyhow!("missing otpks_json"))?;
    let orchestrator_state_bytes = store
        .get_bytes("orchestrator_state")?
        .ok_or_else(|| anyhow::anyhow!("missing orchestrator_state_cfe"))?;

    let private_keys_json = String::from_utf8(private_keys_json_bytes)?;
    let otpks_json = String::from_utf8(otpks_json_bytes)?;

    let keys: PrivateKeysJson = serde_json::from_str(&private_keys_json)?;

    let identity_secret = base64::engine::general_purpose::STANDARD.decode(keys.identity_secret)?;
    let signing_secret = base64::engine::general_purpose::STANDARD.decode(keys.signing_secret)?;
    let prekey_secret = base64::engine::general_purpose::STANDARD
        .decode(keys.signed_prekey_secret)?;
    let prekey_signature = base64::engine::general_purpose::STANDARD
        .decode(keys.prekey_signature)?;

    let client = ClassicClient::<ClassicSuiteProvider>::from_keys(
        identity_secret,
        signing_secret,
        prekey_secret,
        prekey_signature,
    ).map_err(|e| anyhow::anyhow!(e))?;

    let mut orch = Orchestrator::new(client, user_id);
    orch.import_otpks_json(&otpks_json)
        .map_err(|e| anyhow::anyhow!(e))?;
    orch.import_orchestrator_state_cfe(&orchestrator_state_bytes)
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(orch)
}

fn load_user_id(store: &SecureStore) -> anyhow::Result<Option<String>> {
    match store.get_bytes("user_id")? {
        None => Ok(None),
        Some(b) => Ok(Some(String::from_utf8(b)?)),
    }
}

