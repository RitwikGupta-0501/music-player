use tauri::State;
use tokio::sync::oneshot;
use crate::AppState;
use crate::db::DbRequest;
use super::AudioCommand;

#[tauri::command]
pub async fn load_audio(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    let _ = state.db_tx.send(DbRequest::LoadAudioCache { path: path.clone(), resp: tx });
    let _ = rx.await; 

    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::Load(path));
    }
    Ok(())
}

#[tauri::command]
pub async fn play_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::Play);
    }
    Ok(())
}

#[tauri::command]
pub async fn pause_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::Pause);
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::Stop);
    }
    Ok(())
}

#[tauri::command]
pub fn seek_audio(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Seek(position)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::SetVolume(volume));
    }
    Ok(())
}

#[tauri::command]
pub async fn set_mute(state: State<'_, AppState>, mute: bool) -> Result<(), String> {
    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::SetMute(mute));
    }
    Ok(())
}
