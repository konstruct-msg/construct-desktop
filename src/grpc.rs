//! gRPC client layer (tonic) for auth/key/messaging.
//!
//! This module expects `build.rs` (tonic-build) to generate code from `construct-protos`.

pub mod services {
    tonic::include_proto!("shared.proto.services.v1");
}

pub mod core {
    tonic::include_proto!("shared.proto.core.v1");
}

pub mod signaling {
    tonic::include_proto!("shared.proto.signaling.v1");
}

pub mod messaging {
    tonic::include_proto!("shared.proto.messaging.v1");
}

use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, Endpoint};

const SERVER_HOST: &str = "ams.konstruct.cc";
const SERVER_PORT: u16 = 443;

#[derive(Debug, Clone)]
pub struct GrpcConfig {
    pub host: String,
    pub port: u16,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            host: SERVER_HOST.to_string(),
            port: SERVER_PORT,
        }
    }
}

#[derive(Debug)]
pub struct GrpcTransport {
    cfg: GrpcConfig,
    channel: Channel,
}

impl GrpcTransport {
    pub async fn connect(cfg: GrpcConfig) -> anyhow::Result<Self> {
        let url = format!("https://{}:{}", cfg.host, cfg.port);
        let endpoint = Endpoint::from_shared(url)?.tls_config(tonic::transport::ClientTlsConfig::new())?;
        let channel = endpoint.connect().await?;
        Ok(Self { cfg, channel })
    }

    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    pub fn auth_metadata(&self, access_token: &str) -> anyhow::Result<tonic::metadata::MetadataMap> {
        let mut meta = tonic::metadata::MetadataMap::new();
        let v = format!("Bearer {access_token}");
        meta.insert("authorization", MetadataValue::from_str(&v)?);
        Ok(meta)
    }
}

/// AuthService unary API.
pub struct AuthServiceApi {
    client: services::auth_service_client::AuthServiceClient<Channel>,
}

impl AuthServiceApi {
    pub fn new(channel: Channel) -> Self {
        let client = services::auth_service_client::AuthServiceClient::new(channel);
        Self { client }
    }

    pub async fn get_pow_challenge(&mut self) -> anyhow::Result<services::GetPowChallengeResponse> {
        let req = tonic::Request::new(services::GetPowChallengeRequest {});
        let resp = self.client.get_pow_challenge(req).await?;
        Ok(resp.into_inner())
    }

    pub async fn register_device(
        &mut self,
        request: services::RegisterDeviceRequest,
    ) -> anyhow::Result<services::AuthTokensResponse> {
        let req = tonic::Request::new(request);
        let resp = self.client.register_device(req).await?;
        Ok(resp.into_inner())
    }

    pub async fn authenticate_device(
        &mut self,
        request: services::AuthenticateDeviceRequest,
    ) -> anyhow::Result<services::AuthTokensResponse> {
        let req = tonic::Request::new(request);
        let resp = self.client.authenticate_device(req).await?;
        Ok(resp.into_inner())
    }
}

/// KeyService unary API.
pub struct KeyServiceApi {
    client: services::key_service_client::KeyServiceClient<Channel>,
}

impl KeyServiceApi {
    pub fn new(channel: Channel) -> Self {
        let client = services::key_service_client::KeyServiceClient::new(channel);
        Self { client }
    }

    pub async fn get_pre_key_bundle(
        &mut self,
        request: services::GetPreKeyBundleRequest,
        metadata: Option<tonic::metadata::MetadataMap>,
    ) -> anyhow::Result<services::GetPreKeyBundleResponse> {
        let req = tonic::Request::new(request);
        if let Some(m) = metadata {
            *req.metadata_mut() = m;
        }
        let resp = self.client.get_pre_key_bundle(req).await?;
        Ok(resp.into_inner())
    }

    pub async fn get_pre_key_count(
        &mut self,
        request: services::GetPreKeyCountRequest,
        metadata: Option<tonic::metadata::MetadataMap>,
    ) -> anyhow::Result<services::GetPreKeyCountResponse> {
        let mut req = tonic::Request::new(request);
        if let Some(m) = metadata {
            *req.metadata_mut() = m;
        }
        let resp = self.client.get_pre_key_count(req).await?;
        Ok(resp.into_inner())
    }

    pub async fn upload_pre_keys(
        &mut self,
        request: services::UploadPreKeysRequest,
        metadata: Option<tonic::metadata::MetadataMap>,
    ) -> anyhow::Result<services::UploadPreKeysResponse> {
        let mut req = tonic::Request::new(request);
        if let Some(m) = metadata {
            *req.metadata_mut() = m;
        }
        let resp = self.client.upload_pre_keys(req).await?;
        Ok(resp.into_inner())
    }
}

/// MessagingService unary API (MVP: polling via `GetPendingMessages`).
pub struct MessagingApi {
    client: services::messaging_service_client::MessagingServiceClient<Channel>,
}

impl MessagingApi {
    pub fn new(channel: Channel) -> Self {
        let client = services::messaging_service_client::MessagingServiceClient::new(channel);
        Self { client }
    }

    pub async fn send_message(
        &mut self,
        request: services::SendMessageRequest,
        metadata: Option<tonic::metadata::MetadataMap>,
    ) -> anyhow::Result<services::SendMessageResponse> {
        let mut req = tonic::Request::new(request);
        if let Some(m) = metadata {
            *req.metadata_mut() = m;
        }
        let resp = self.client.send_message(req).await?;
        Ok(resp.into_inner())
    }

    pub async fn get_pending_messages(
        &mut self,
        request: services::GetPendingMessagesRequest,
        metadata: Option<tonic::metadata::MetadataMap>,
    ) -> anyhow::Result<services::GetPendingMessagesResponse> {
        let mut req = tonic::Request::new(request);
        if let Some(m) = metadata {
            *req.metadata_mut() = m;
        }
        let resp = self.client.get_pending_messages(req).await?;
        Ok(resp.into_inner())
    }
}

