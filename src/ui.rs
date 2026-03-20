use crate::engine::{ChatMessage, ChatSummary, SendTextResult};
use crate::engine::ConstructEngine;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppStatus {
    pub is_authenticated: bool,
    pub user_id: Option<String>,
}

#[tauri::command]
pub async fn get_app_status(engine: tauri::State<'_, ConstructEngine>) -> Result<AppStatus, String> {
    let state = engine.state.lock().await;
    Ok(AppStatus {
        is_authenticated: state.is_authenticated,
        user_id: state.user_id.clone(),
    })
}

#[tauri::command]
pub async fn list_chats(engine: tauri::State<'_, ConstructEngine>) -> Result<Vec<ChatSummary>, String> {
    let state = engine.state.lock().await;
    Ok(state.chats.clone())
}

#[tauri::command]
pub async fn list_messages(
    engine: tauri::State<'_, ConstructEngine>,
    chat_id: String,
) -> Result<Vec<ChatMessage>, String> {
    let state = engine.state.lock().await;
    Ok(state.messages.get(&chat_id).cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn send_text(
    engine: tauri::State<'_, ConstructEngine>,
    chat_id: String,
    text: String,
) -> Result<SendTextResult, String> {
    engine.send_text(chat_id, text).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ensure_authenticated(
    engine: tauri::State<'_, ConstructEngine>,
    username: Option<String>,
) -> Result<AppStatus, String> {
    engine
        .ensure_device_auth(username)
        .await
        .map_err(|e| e.to_string())?;
    // Start background receiver so incoming messages show up in UI.
    let _ = engine.start_receive_loop().await;
    let state = engine.state.lock().await;
    Ok(AppStatus {
        is_authenticated: state.is_authenticated,
        user_id: state.user_id.clone(),
    })
}

