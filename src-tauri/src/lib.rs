use serde::{Serialize, Deserialize};
use std::sync::mpsc::{self, Sender};
use tauri::{AppHandle, Manager, State, RunEvent};
use walkdir::WalkDir;
use tokio::sync::oneshot;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;
pub mod db;
pub mod audio;
pub mod providers;
pub mod queue;
pub mod telemetry;
pub mod feature_flags;

use providers::{ProviderManager, TrackResult};
use audio::AudioCommand;
use db::{DbRequest, TrackData};
use queue::QueueState;

#[derive(Serialize, Deserialize, Clone)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist: Option<String>,
    pub cover_art_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalTrack {
    pub id: i64,
    pub title: String,
    pub artist: Option<String>,
    pub album_id: Option<i64>,
    pub track_number: Option<i64>,
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
}

pub struct AppState {
    pub audio_tx: std::sync::Mutex<Sender<AudioCommand>>,
    pub db_tx: std::sync::mpsc::Sender<DbRequest>,
    pub provider_manager: tokio::sync::Mutex<ProviderManager>,
    pub audio_thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    pub db_thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    pub queue: std::sync::Mutex<QueueState>,
    pub reqwest_client: reqwest::Client,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub file_path: String,
    pub status: String,
    pub error_message: Option<String>,
    pub checksum: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub homepage: Option<String>,
    pub settings_schema: Option<String>,
    pub priority: i32,
    pub icon: Option<String>,
    pub settings: Option<String>,
}

#[tauri::command]
async fn sync_providers(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut providers = Vec::new();
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let providers_dir = app_data_dir.join("providers");
    
    if !providers_dir.exists() {
        let _ = std::fs::create_dir_all(&providers_dir);
    }
    
    // We parse the Lua metadata using regex to avoid spinning up a full VM for every file
    let re_id = regex::Regex::new(r#"id\s*=\s*"([^"]+)""#).unwrap();
    let re_name = regex::Regex::new(r#"name\s*=\s*"([^"]+)""#).unwrap();
    let re_author = regex::Regex::new(r#"author\s*=\s*"([^"]+)""#).unwrap();
    let re_version = regex::Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
    let re_homepage = regex::Regex::new(r#"homepage\s*=\s*"([^"]+)""#).unwrap();
    let re_settings = regex::Regex::new(r#"settings_schema\s*=\s*"([^"]+)""#).unwrap();
    let re_priority = regex::Regex::new(r#"priority\s*=\s*([0-9]+)"#).unwrap();
    let re_icon = regex::Regex::new(r#"icon\s*=\s*"([^"]+)""#).unwrap();
    // Simplified capabilities parser - expects a simple lua array of strings
    let re_capabilities = regex::Regex::new(r#"capabilities\s*=\s*\{([^}]+)\}"#).unwrap();

    if let Ok(entries) = std::fs::read_dir(&providers_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("lua") {
                let fallback_id = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                let checksum = Some(hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>());
                
                let id = re_id.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or(fallback_id);
                let name = re_name.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or(id.clone());
                let author = re_author.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or_else(|| "Unknown".to_string());
                let version = re_version.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string()).unwrap_or_else(|| "0.0.0".to_string());
                let homepage = re_homepage.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
                let settings_schema = re_settings.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
                let priority = re_priority.captures(&content).and_then(|c| c.get(1)).and_then(|m| m.as_str().parse::<i32>().ok()).unwrap_or(0);
                let icon = re_icon.captures(&content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
                
                let capabilities = re_capabilities.captures(&content).and_then(|c| c.get(1)).map(|m| {
                    m.as_str().split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<String>>()
                });
                
                providers.push(ProviderInfo {
                    id,
                    name,
                    author,
                    version,
                    file_path: path.to_string_lossy().into_owned(),
                    status: "enabled".to_string(), // Default when inserting, preserved on update by SQL
                    error_message: None,
                    checksum,
                    capabilities,
                    homepage,
                    settings_schema,
                    priority,
                    icon,
                    settings: None,
                });
            }
        }
    }

    let (tx, rx) = tokio::sync::oneshot::channel();
    state.db_tx.send(crate::db::DbRequest::SyncProviders { providers, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())??;
    
    Ok(())
}

#[tauri::command]
async fn get_providers(state: State<'_, AppState>) -> Result<Vec<ProviderInfo>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state.db_tx.send(crate::db::DbRequest::GetProviders { resp: tx }).map_err(|e| e.to_string())?;
    let providers = rx.await.map_err(|e| e.to_string())??;
    
    let mut manager = state.provider_manager.lock().await;
    manager.sync_registry(providers.clone());
    
    Ok(providers)
}

#[tauri::command]
async fn toggle_provider(
    provider_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(crate::db::DbRequest::ToggleProvider { 
        provider_id, 
        enabled, 
        resp: tx 
    }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())??;
    Ok(())
}

#[tauri::command]
async fn save_provider_settings(
    provider_id: String,
    settings_json: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(crate::db::DbRequest::SaveProviderSettings {
        provider_id,
        settings_json,
        resp: tx,
    }).map_err(|e| e.to_string())?;
    
    rx.await.map_err(|e| e.to_string())??;
    Ok(())
}

#[tauri::command]
async fn search_provider(state: State<'_, AppState>, provider_id: String, query: String) -> Result<Vec<TrackResult>, String> {
    let manager = state.provider_manager.lock().await;
    let results = manager
        .search(&provider_id, &query)
        .await
        .map_err(|e| e.to_string())?;

    Ok(results)
}

#[tauri::command]
async fn get_provider_config(
    state: State<'_, AppState>,
    provider_id: String,
    key: String,
) -> Result<Option<String>, String> {
    let secret_store = providers::secrets::ProviderSecretStore::new();
    Ok(secret_store.get(&provider_id, &key, &state.db_tx))
}

#[tauri::command]
async fn set_provider_config(
    state: State<'_, AppState>,
    provider_id: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let secret_store = providers::secrets::ProviderSecretStore::new();
    secret_store.set(&provider_id, &key, &value, &state.db_tx)
}

#[tauri::command]
async fn search_library(state: State<'_, AppState>, query: String, limit: u32) -> Result<Vec<LocalTrack>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state.db_tx.send(DbRequest::SearchLibrary { query, limit, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn fuzzy_match_tracks(local_track: LocalTrack, remote_tracks: Vec<providers::TrackResult>) -> Vec<providers::TrackResult> {
    let mut matches = Vec::new();
    let local_title = local_track.title.to_lowercase();
    let local_artist = local_track.artist.unwrap_or_default().to_lowercase();

    for remote in remote_tracks {
        let remote_title = remote.title.to_lowercase();
        let remote_artist = remote.artist.to_lowercase();
        
        let title_sim = strsim::jaro_winkler(&local_title, &remote_title);
        let artist_sim = strsim::jaro_winkler(&local_artist, &remote_artist);
        
        if title_sim > 0.85 && artist_sim > 0.85 {
            matches.push(remote);
        }
    }
    matches
}

#[tauri::command]
async fn scan_local_directory(state: State<'_, AppState>, path: String) -> Result<usize, String> {
    let path_clone = path.clone();
    
    let tracks = tokio::task::spawn_blocking(move || {
        let mut found_tracks = Vec::new();
        let cleaner = regex::Regex::new(r"(?i)\s*(?:\[[^\]]*\]|\([^\)]*\))").unwrap();
        
        for entry in WalkDir::new(path_clone).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(ext) = entry_path.extension().and_then(|s| s.to_str()) {
                    let ext = ext.to_lowercase();
                    if ext == "mp3" || ext == "flac" || ext == "wav" || ext == "m4a" || ext == "ogg" {
                        let mut title = entry_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let mut artist = None;
                        let mut album = None;
                        let mut track_number = None;
                        if let Ok(file) = std::fs::File::open(entry_path) {
                            let mss = MediaSourceStream::new(Box::new(file), Default::default());
                            let mut hint = Hint::new();
                            hint.with_extension(&ext);

                            if let Ok(mut probed) = symphonia::default::get_probe().format(
                                &hint,
                                mss,
                                &FormatOptions::default(),
                                &MetadataOptions::default(),
                            ) {
                                // Try format metadata first, then global metadata
                                let mut meta_opt = probed.format.metadata().current().cloned();
                                if meta_opt.is_none() {
                                    meta_opt = probed.metadata.get().as_ref().and_then(|m| m.current()).cloned();
                                }
                                
                                if let Some(metadata) = meta_opt {
                                    for tag in metadata.tags() {
                                        match tag.std_key {
                                            Some(StandardTagKey::TrackTitle) => {
                                                let t = cleaner.replace_all(&tag.value.to_string(), "").trim().to_string();
                                                title = if t.is_empty() { tag.value.to_string() } else { t };
                                            },
                                            Some(StandardTagKey::Artist) => artist = Some(tag.value.to_string()),
                                            Some(StandardTagKey::Album) => {
                                                let a = cleaner.replace_all(&tag.value.to_string(), "").trim().to_string();
                                                album = Some(if a.is_empty() { tag.value.to_string() } else { a });
                                            },
                                            Some(StandardTagKey::TrackNumber) => {
                                                let val = tag.value.to_string();
                                                if let Ok(num) = val.parse::<i64>() {
                                                    track_number = Some(num);
                                                } else if let Some(num_str) = val.split('/').next() {
                                                    if let Ok(num) = num_str.parse::<i64>() {
                                                        track_number = Some(num);
                                                    }
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        
                        found_tracks.push(TrackData {
                            title, artist, album, track_number, file_path: entry_path.to_string_lossy().to_string()
                        });
                    }
                }
            }
        }
        found_tracks
    }).await.map_err(|e| e.to_string())?;

    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::InsertTracks { tracks, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_local_tracks(state: State<'_, AppState>, limit: u32, offset: u32) -> Result<Vec<LocalTrack>, String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::GetLocalTracks { limit, offset, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_albums(state: State<'_, AppState>, limit: u32, offset: u32) -> Result<Vec<Album>, String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::GetAlbums { limit, offset, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_album_tracks(state: State<'_, AppState>, album_id: i64, limit: u32, offset: u32) -> Result<Vec<LocalTrack>, String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::GetAlbumTracks { album_id, limit, offset, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_playlists(state: State<'_, AppState>, limit: u32, offset: u32) -> Result<Vec<Playlist>, String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::GetPlaylists { limit, offset, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn create_playlist(state: State<'_, AppState>, name: String) -> Result<i64, String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::CreatePlaylist { name, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn add_to_playlist(state: State<'_, AppState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::AddToPlaylist { playlist_id, track_id, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_playlist_tracks(state: State<'_, AppState>, playlist_id: i64, limit: u32, offset: u32) -> Result<Vec<LocalTrack>, String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::GetPlaylistTracks { playlist_id, limit, offset, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remove_from_playlist(state: State<'_, AppState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::RemoveFromPlaylist { playlist_id, track_id, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn delete_playlist(state: State<'_, AppState>, playlist_id: i64) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::DeletePlaylist { playlist_id, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn rename_playlist(state: State<'_, AppState>, playlist_id: i64, new_name: String) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::RenamePlaylist { playlist_id, new_name, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn reorder_playlist_track(state: State<'_, AppState>, playlist_id: i64, from_pos: i64, to_pos: i64) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::ReorderPlaylistTrack { playlist_id, from_pos, to_pos, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn clear_local_library(state: State<'_, AppState>) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::ClearLocalLibrary { resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::GetSetting { key, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::SetSetting { key, value, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remove_track_by_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::RemoveTrackByPath { path, resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn factory_reset(app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (tx, rx) = oneshot::channel();
    state.db_tx.send(DbRequest::FactoryReset { resp: tx }).map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())??;

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let providers_dir = app_data_dir.join("providers");
    let artwork_dir = app_data_dir.join("artwork");
    
    let _ = std::fs::remove_dir_all(&providers_dir);
    let _ = std::fs::remove_dir_all(&artwork_dir);

    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::Stop);
    }
    
    // Also clear the queue state explicitly
    if let Ok(mut q) = state.queue.lock() {
        let _ = q.clear();
    }

    Ok(())
}

#[tauri::command]
async fn extract_and_cache_artwork(app_handle: AppHandle, track_id: i64, file_path: String) -> Result<Option<String>, String> {
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let artwork_dir = app_data_dir.join("artwork");
    std::fs::create_dir_all(&artwork_dir).map_err(|e| e.to_string())?;

    let dest_path = artwork_dir.join(format!("{}.jpg", track_id));
    if dest_path.exists() {
        return Ok(Some(dest_path.to_string_lossy().to_string()));
    }

    let result = tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
        if let Ok(file) = std::fs::File::open(&file_path) {
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            let mut hint = Hint::new();
            if let Some(ext) = std::path::Path::new(&file_path).extension().and_then(|e| e.to_str()) {
                hint.with_extension(ext);
            }

            if let Ok(mut probed) = symphonia::default::get_probe().format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            ) {
                let mut visual_data: Option<Vec<u8>> = None;
                
                if let Some(meta) = probed.format.metadata().current() {
                    if let Some(v) = meta.visuals().first() {
                        visual_data = Some(v.data.to_vec());
                    }
                }
                
                if visual_data.is_none() {
                    if let Some(global) = probed.metadata.get().as_ref() {
                        if let Some(meta) = global.current() {
                            if let Some(v) = meta.visuals().first() {
                                visual_data = Some(v.data.to_vec());
                            }
                        }
                    }
                }
                
                if let Some(data) = visual_data {
                    if std::fs::write(&dest_path, &data).is_ok() {
                        return Ok(Some(dest_path.to_string_lossy().to_string()));
                    }
                }
            }
        }
        Ok(None)
    }).await.map_err(|e| e.to_string())?;

    result
}

#[tauri::command]
fn validate_queue_reorder(from_index: u32, to_index: u32, queue_length: u32) -> Result<(), String> {
    if from_index >= queue_length {
        return Err(format!("Invalid source index: {}", from_index));
    }
    if to_index >= queue_length {
        return Err(format!("Invalid target index: {}", to_index));
    }
    if from_index == to_index {
        return Err("Source and target indices are the same".to_string());
    }
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════════
// TELEMETRY & DIAGNOSTICS COMMANDS
// ════════════════════════════════════════════════════════════════════════════════

#[tauri::command]
fn get_error_log() -> Vec<telemetry::ErrorEvent> {
    telemetry::get_error_log()
}

#[tauri::command]
fn get_error_count() -> u64 {
    telemetry::error_count()
}

#[tauri::command]
fn clear_error_log() {
    telemetry::clear_error_log();
    log::info!("Error log cleared");
}

// ════════════════════════════════════════════════════════════════════════════════
// FEATURE FLAG COMMANDS
// ════════════════════════════════════════════════════════════════════════════════

#[tauri::command]
fn get_feature_flags() -> Vec<String> {
    feature_flags::FEATURE_FLAGS
        .get_enabled_flags()
        .iter()
        .map(|f| format!("{:?}", f))
        .collect()
}

#[tauri::command]
fn is_feature_enabled(flag: String) -> bool {
    match flag.as_str() {
        "ShuffleMode" => feature_flags::FEATURE_FLAGS.is_enabled(feature_flags::FeatureFlag::ShuffleMode),
        "ReorderQueue" => feature_flags::FEATURE_FLAGS.is_enabled(feature_flags::FeatureFlag::ReorderQueue),
        "VirtualScrolling" => feature_flags::FEATURE_FLAGS.is_enabled(feature_flags::FeatureFlag::VirtualScrolling),
        "PersistentQueue" => feature_flags::FEATURE_FLAGS.is_enabled(feature_flags::FeatureFlag::PersistentQueue),
        _ => false,
    }
}

#[tauri::command]
fn set_feature_enabled(flag: String, enabled: bool) {
    let feature = match flag.as_str() {
        "ShuffleMode" => feature_flags::FeatureFlag::ShuffleMode,
        "ReorderQueue" => feature_flags::FeatureFlag::ReorderQueue,
        "VirtualScrolling" => feature_flags::FeatureFlag::VirtualScrolling,
        "PersistentQueue" => feature_flags::FeatureFlag::PersistentQueue,
        _ => return,
    };

    if enabled {
        feature_flags::FEATURE_FLAGS.enable(feature.clone());
        log::info!("Feature enabled: {:?}", feature);
    } else {
        feature_flags::FEATURE_FLAGS.disable(feature.clone());
        log::info!("Feature disabled: {:?}", feature);
    }
}

#[tauri::command]
async fn sync_playback_state(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.audio_tx.lock() {
        let _ = tx.send(AudioCommand::SyncState);
    }
    Ok(())
}

#[tauri::command]
fn open_in_file_explorer(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_millis()
        .try_init()
        .ok();

    log::info!("Echo Music Player starting up");

    let (audio_tx, audio_rx) = mpsc::channel();
    let (db_tx, db_rx) = mpsc::channel();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            
            // Setup shared reqwest client with strict redirect/SSRF policy and timeouts
            let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    return attempt.stop();
                }
                if crate::providers::check_url_allowed(attempt.url().as_str()).is_err() {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            });
            let reqwest_client = reqwest::Client::builder()
                .redirect(redirect_policy)
                .timeout(std::time::Duration::from_secs(30))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to build reqwest client");

            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let db_dir = app_data_dir.join("database");
            let providers_dir = app_data_dir.join("providers");
            
            std::fs::create_dir_all(&db_dir).expect("Failed to create database directory");
            std::fs::create_dir_all(&providers_dir).expect("Failed to create providers directory");
            
            let db_path = db_dir.join("echo_library.db");
            let conn = db::schema::init_db(&db_path).expect("Failed to initialize SQLite");

            // Recover queue state on startup (Phase 5)
            let recovered_queue = queue::recovery::recover_on_startup(&conn)
                .unwrap_or_else(|e| {
                    log::warn!("Queue recovery failed: {}, starting fresh", e);
                    telemetry::record_error("queue_recovery", &e);
                    QueueState::new()
                });

            log::info!("Queue initialized with {} tracks", recovered_queue.tracks.len());

            let db_thread_handle = db::start_db_thread(conn, db_rx);
            
            if let Ok(entries) = std::fs::read_dir(&providers_dir) {
                if entries.count() == 0 {
                    if let Ok(resource_dir) = app.path().resource_dir() {
                        let bundled_providers = resource_dir.join("providers");
                        if bundled_providers.exists() {
                            if let Ok(bundled_entries) = std::fs::read_dir(bundled_providers) {
                                for entry in bundled_entries.flatten() {
                                    if entry.path().is_file() {
                                        if let Some(filename) = entry.file_name().to_str() {
                                            let dest_path = providers_dir.join(filename);
                                            let _ = std::fs::copy(entry.path(), dest_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let provider_manager = ProviderManager::new(reqwest_client.clone());
            let audio_thread_handle = audio::start_audio_thread(audio_rx, handle, reqwest_client.clone(), tauri::async_runtime::handle());

            app.manage(AppState {
                audio_tx: std::sync::Mutex::new(audio_tx),
                db_tx,
                provider_manager: tokio::sync::Mutex::new(provider_manager),
                audio_thread: std::sync::Mutex::new(Some(audio_thread_handle)),
                db_thread: std::sync::Mutex::new(Some(db_thread_handle)),
                queue: std::sync::Mutex::new(recovered_queue),
                reqwest_client,
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            audio::commands::load_audio,
            audio::commands::play_audio,
            audio::commands::pause_audio,
            audio::commands::stop_audio,
            audio::commands::seek_audio,
            audio::commands::set_volume,
            audio::commands::set_mute,
            get_providers,
            toggle_provider,
            save_provider_settings,
            sync_providers,
            get_provider_config,
            set_provider_config,
            search_provider,
            search_library,
            fuzzy_match_tracks,
            scan_local_directory,
            get_local_tracks,
            get_albums,
            get_album_tracks,
            get_playlists,
            create_playlist,
            add_to_playlist,
            get_playlist_tracks,
            remove_from_playlist,
            delete_playlist,
            rename_playlist,
            reorder_playlist_track,
            validate_queue_reorder,
            extract_and_cache_artwork,
            clear_local_library,
            get_setting,
            set_setting,
            factory_reset,
            remove_track_by_path,
            // Queue commands (Phase 2)
            queue::commands::set_queue,
            queue::commands::add_to_queue,
            queue::commands::clear_queue,
            queue::commands::skip_forward,
            queue::commands::skip_backward,
            queue::commands::jump_to_position,
            queue::commands::jump_to_track,
            queue::commands::reorder_queue,
            queue::commands::set_repeat_mode,
            queue::commands::set_shuffle,
            queue::commands::get_queue,
            queue::commands::get_queue_length,
            queue::commands::get_current_track,
            queue::commands::reshuffle,
            // Telemetry commands (Phase 7)
            get_error_log,
            get_error_count,
            clear_error_log,
            // Feature flag commands (Phase 7)
            get_feature_flags,
            is_feature_enabled,
            set_feature_enabled,
            toggle_provider,
            sync_playback_state,
            open_in_file_explorer,
        ]);

    builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                let state: State<'_, AppState> = app.state();
                if let Ok(tx) = state.audio_tx.lock() {
                    let _ = tx.send(AudioCommand::Quit);
                }
                let _ = state.db_tx.send(DbRequest::Quit);
                
                if let Ok(mut lock) = state.audio_thread.lock() {
                    if let Some(handle) = lock.take() {
                        let _ = handle.join();
                    }
                };
                
                if let Ok(mut lock) = state.db_thread.lock() {
                    if let Some(handle) = lock.take() {
                        let _ = handle.join();
                    }
                };
            }
        });
}
