use serde::{Serialize, Deserialize};
use std::sync::mpsc::{self, Sender};
use tauri::{AppHandle, Manager, State, RunEvent};
use walkdir::WalkDir;
use tokio::sync::oneshot;
use id3::TagLike;

pub mod db;
pub mod audio;
pub mod providers;

use providers::{ProviderManager, TrackResult};
use audio::AudioCommand;
use db::{DbRequest, TrackData};

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
}

#[tauri::command]
async fn search_provider(state: State<'_, AppState>, query: String) -> Result<Vec<TrackResult>, String> {
    let manager = state.provider_manager.lock().await;
    let results = manager
        .search(&query)
        .await
        .map_err(|e| e.to_string())?;

    Ok(results)
}

#[tauri::command]
async fn scan_local_directory(state: State<'_, AppState>, path: String) -> Result<usize, String> {
    let path_clone = path.clone();
    
    let tracks = tokio::task::spawn_blocking(move || {
        let mut found_tracks = Vec::new();
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
                        
                        if let Ok(tag) = id3::Tag::read_from_path(entry_path) {
                            if let Some(tag_title) = tag.title() { title = tag_title.to_string(); }
                            if let Some(tag_artist) = tag.artist() { artist = Some(tag_artist.to_string()); }
                            if let Some(tag_album) = tag.album() { album = Some(tag_album.to_string()); }
                            if let Some(tag_track) = tag.track() { track_number = Some(tag_track as i64); }
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
    let _ = std::fs::remove_dir_all(&providers_dir);

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
        if let Ok(tag) = id3::Tag::read_from_path(&file_path) {
            if let Some(pic) = tag.pictures().next() {
                if std::fs::write(&dest_path, &pic.data).is_ok() {
                    return Ok(Some(dest_path.to_string_lossy().to_string()));
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (audio_tx, audio_rx) = mpsc::channel();
    let (db_tx, db_rx) = mpsc::channel();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let db_dir = app_data_dir.join("database");
            let providers_dir = app_data_dir.join("providers");
            
            std::fs::create_dir_all(&db_dir).expect("Failed to create database directory");
            std::fs::create_dir_all(&providers_dir).expect("Failed to create providers directory");
            
            let db_path = db_dir.join("echo_library.db");
            let conn = db::schema::init_db(&db_path).expect("Failed to initialize SQLite");
            
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

            let dummy_script = providers_dir.join("dummy_search.lua");
            // Fallback to empty string if dummy_search doesn't exist to prevent crash on boot
            if !dummy_script.exists() {
                let _ = std::fs::write(&dummy_script, "return { search = function(q) return {} end }");
            }
            let provider_manager = ProviderManager::new(&dummy_script).expect("Failed to init Lua sandbox");
            let audio_thread_handle = audio::start_audio_thread(audio_rx, handle);

            app.manage(AppState {
                audio_tx: std::sync::Mutex::new(audio_tx),
                db_tx,
                provider_manager: tokio::sync::Mutex::new(provider_manager),
                audio_thread: std::sync::Mutex::new(Some(audio_thread_handle)),
                db_thread: std::sync::Mutex::new(Some(db_thread_handle)),
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
            search_provider,
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
