use tauri::State;
use crate::AppState;
use super::AudioCommand;

#[tauri::command]
pub async fn load_audio(
    state: State<'_, AppState>,
    source: crate::queue::TrackSourceInfo,
    title: String,
    artist: Option<String>,
    album: Option<String>,
) -> Result<(), String> {
    let (resolved_source, duration_hint) = match source {
        crate::queue::TrackSourceInfo::Local { file_path, .. } => {
            let pb = std::path::PathBuf::from(&file_path);
            if !pb.exists() {
                return Err("FILE_NOT_FOUND".to_string());
            }
            (crate::audio::TrackSource::Local(pb), None)
        }
        crate::queue::TrackSourceInfo::Remote { provider_id, remote_track_id, stream_url, duration_ms, .. } => {
            let final_url = if let Some(url) = stream_url {
                url
            } else {
                // Needs resolution
                let manager = state.provider_manager.lock().await;
                let resolved = manager.resolve(&provider_id, &remote_track_id).await.map_err(|e| e.to_string())?;
                resolved.stream_url
            };
            
            crate::providers::check_url_allowed(&final_url).map_err(|e| e.to_string())?;
            let parsed_url = url::Url::parse(&final_url).map_err(|e| format!("Invalid URL: {}", e))?;
            (crate::audio::TrackSource::Remote(parsed_url), duration_ms)
        }
    };

    // We skip LoadAudioCache since it's just for local files.
    // In a real app we'd do a smarter history log here instead.

    let tx = state.audio_tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Load { source: resolved_source, title, artist, album, duration_hint }).map_err(|e| e.to_string())?;
    
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
