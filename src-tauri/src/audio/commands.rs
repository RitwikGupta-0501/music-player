use tauri::State;
use tokio::sync::oneshot;
use crate::AppState;
use crate::db::DbRequest;
use super::AudioCommand;

#[tauri::command]
pub async fn load_audio(
    state: State<'_, AppState>,
    path: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
) -> Result<(), String> {
    if !std::path::Path::new(&path).exists() {
        return Err("FILE_NOT_FOUND".to_string());
    }

    let (tx, rx) = oneshot::channel();
    let _ = state.db_tx.send(DbRequest::LoadAudioCache { path: path.clone(), resp: tx });
    let _ = rx.await;

    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Load { path, title, artist, album }).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn play_audio(state: State<'_, AppState>) -> Result<(), String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Play).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn pause_audio(state: State<'_, AppState>) -> Result<(), String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Pause).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn stop_audio(state: State<'_, AppState>) -> Result<(), String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Stop).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn seek_audio(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Seek(position)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::SetVolume(volume)).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_mute(state: State<'_, AppState>, mute: bool) -> Result<(), String> {
    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::SetMute(mute)).map_err(|e| e.to_string())?;
    Ok(())
}
