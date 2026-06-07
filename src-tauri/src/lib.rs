use rodio::{Decoder, OutputStream, Sink};
use rusqlite::{Connection, Result as SqlResult};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, Manager, State};
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};
use id3::TagLike;

mod providers;
use providers::{ProviderManager, TrackResult};

#[derive(Serialize, Deserialize)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist: Option<String>,
    pub cover_art_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LocalTrack {
    pub id: i64,
    pub title: String,
    pub artist: Option<String>,
    pub album_id: Option<i64>,
    pub track_number: Option<i64>,
    pub file_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Clone)]
struct PlayerSync {
    state: String,
    position: f64,
    duration: f64,
    track: String,
}

// 1. Message Payload
pub enum AudioCommand {
    Load(String),
    Play,
    Pause,
    Stop,
    Seek(f64),
    SetVolume(f32),
    SetMute(bool),
}

// 2. Global State
struct AppState {
    tx: Mutex<Sender<AudioCommand>>,
    db_conn: Mutex<Connection>,
}

// 3. Database Initialization
fn init_db<P: AsRef<std::path::Path>>(db_path: P) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Removed DB drop logic to persist user data
    // Tables will be created below if they don't exist

    conn.execute(
        "CREATE TABLE IF NOT EXISTS albums (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT,
            cover_art_path TEXT,
            UNIQUE(title, artist)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT,
            album_id INTEGER,
            track_number INTEGER,
            file_path TEXT UNIQUE NOT NULL,
            FOREIGN KEY(album_id) REFERENCES albums(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS playlist_tracks (
            id INTEGER PRIMARY KEY,
            playlist_id INTEGER,
            track_id INTEGER,
            position INTEGER,
            FOREIGN KEY(playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
            FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

// 4. The Audio Thread (Interpolation Strategy: emit sync events on state changes only)
fn start_audio_thread(rx: Receiver<AudioCommand>, app_handle: AppHandle) {
    thread::spawn(move || {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get audio output");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

        let mut current_track_path = String::new();
        let mut current_duration: f64 = 0.0;
        let mut current_volume: f32 = 1.0;
        let mut is_muted: bool = false;

        // Helper closure to emit a single structured sync event
        let emit_sync = |handle: &AppHandle, state: &str, sink: &Sink, track: &str, duration: f64| {
            let _ = handle.emit("player-sync", PlayerSync {
                state: state.to_string(),
                position: sink.get_pos().as_secs_f64(),
                duration,
                track: track.to_string(),
            });
        };

        loop {
            // Block on the channel — zero CPU when idle
            if let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::Load(path) => {
                        sink.stop();
                        if let Ok(file) = File::open(&path) {
                            let reader = BufReader::new(file);
                            if let Ok(decoder) = Decoder::new(reader) {
                                use rodio::Source;
                                current_duration = decoder.total_duration()
                                    .map(|d| d.as_secs_f64())
                                    .unwrap_or(0.0);
                                
                                sink.append(decoder);
                                sink.play();
                                current_track_path = path;
                                emit_sync(&app_handle, "Playing", &sink, &current_track_path, current_duration);
                            }
                        }
                    }
                    AudioCommand::Play => {
                        sink.play();
                        emit_sync(&app_handle, "Playing", &sink, &current_track_path, current_duration);
                    }
                    AudioCommand::Pause => {
                        sink.pause();
                        emit_sync(&app_handle, "Paused", &sink, &current_track_path, current_duration);
                    }
                    AudioCommand::Stop => {
                        sink.stop();
                        current_track_path.clear();
                        current_duration = 0.0;
                        emit_sync(&app_handle, "Stopped", &sink, "", 0.0);
                    }
                    AudioCommand::Seek(pos) => {
                        let _ = sink.try_seek(std::time::Duration::from_secs_f64(pos));
                        let state_str = if sink.is_paused() { "Paused" } else { "Playing" };
                        // Emit the *requested* position, not sink.get_pos() which is stale after try_seek
                        let _ = app_handle.emit("player-sync", PlayerSync {
                            state: state_str.to_string(),
                            position: pos,
                            duration: current_duration,
                            track: current_track_path.clone(),
                        });
                    }
                    AudioCommand::SetVolume(vol) => {
                        current_volume = vol;
                        if !is_muted {
                            sink.set_volume(current_volume);
                        }
                    }
                    AudioCommand::SetMute(muted) => {
                        is_muted = muted;
                        if is_muted {
                            sink.set_volume(0.0);
                        } else {
                            sink.set_volume(current_volume);
                        }
                    }
                }
            }
        }
    });
}

#[tauri::command]
async fn search_provider(app_handle: AppHandle, query: String) -> Result<Vec<TrackResult>, String> {
    // In a production app, we would load the manager once into Tauri state,
    // but for testing the bridge, we spin it up on demand.
    let manager = ProviderManager::new().map_err(|e| e.to_string())?;

    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let provider_path = app_data_dir.join("providers").join("dummy_search.lua");

    // We point this to our dynamically resolved path
    let results = manager
        .search(provider_path.to_string_lossy().as_ref(), &query)
        .await
        .map_err(|e| e.to_string())?;

    Ok(results)
}

#[tauri::command]
async fn scan_local_directory(state: State<'_, AppState>, path: String) -> Result<usize, String> {
    let path_clone = path.clone();
    
    // Perform file system scanning in a blocking task so we don't block the async runtime
    let tracks = tokio::task::spawn_blocking(move || {
        let mut found_tracks = Vec::new();
        for entry in WalkDir::new(path_clone).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(ext) = entry_path.extension().and_then(|s| s.to_str()) {
                    let ext = ext.to_lowercase();
                    if ext == "mp3" || ext == "flac" || ext == "wav" {
                        let mut title = entry_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let mut artist = None;
                        let mut album = None;
                        let mut track_number = None;
                        
                        // Try to extract ID3 tags
                        if let Ok(tag) = id3::Tag::read_from_path(entry_path) {
                            if let Some(tag_title) = tag.title() {
                                title = tag_title.to_string();
                            }
                            if let Some(tag_artist) = tag.artist() {
                                artist = Some(tag_artist.to_string());
                            }
                            if let Some(tag_album) = tag.album() {
                                album = Some(tag_album.to_string());
                            }
                            if let Some(tag_track) = tag.track() {
                                track_number = Some(tag_track as i64);
                            }
                        }
                        
                        found_tracks.push((title, artist, album, track_number, entry_path.to_string_lossy().to_string()));
                    }
                }
            }
        }
        found_tracks
    }).await.map_err(|e| e.to_string())?;

    let mut added = 0;
    if let Ok(mut conn) = state.db_conn.lock() {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for (title, artist, album, track_number, file_path) in tracks {
            println!("Processing found track: {} ({})", title, file_path);
            let mut album_id: Option<i64> = None;
            
            // Use fallback for missing album/artist info so they show up in the UI
            let album_title = album.unwrap_or_else(|| "Unknown Album".to_string());
            let album_artist = artist.clone().unwrap_or_else(|| "Unknown Artist".to_string());

            // Insert or get album
            let _ = tx.execute(
                "INSERT OR IGNORE INTO albums (title, artist) VALUES (?1, ?2)",
                (&album_title, &album_artist),
            );
            
            // Get the album ID
            if let Ok(mut stmt) = tx.prepare("SELECT id FROM albums WHERE title = ?1 AND artist = ?2") {
                if let Ok(mut rows) = stmt.query((&album_title, &album_artist)) {
                    if let Ok(Some(row)) = rows.next() {
                        album_id = row.get(0).ok();
                    }
                }
            }

            let res = tx.execute(
                "INSERT OR IGNORE INTO tracks (title, artist, album_id, track_number, file_path) VALUES (?1, ?2, ?3, ?4, ?5)",
                (&title, &artist, &album_id, &track_number, &file_path),
            );
            
            if res.is_ok() && res.unwrap() > 0 {
                println!("Successfully added to DB: {}", title);
                added += 1;
            } else {
                println!("Track already exists or failed to insert: {}", title);
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
    }
    
    Ok(added)
}

#[tauri::command]
async fn get_local_tracks(state: State<'_, AppState>) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT id, title, artist, album_id, track_number, file_path FROM tracks").map_err(|e| e.to_string())?;
        let track_iter = stmt.query_map([], |row| {
            Ok(LocalTrack {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album_id: row.get(3)?,
                track_number: row.get(4)?,
                file_path: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        for track in track_iter {
            if let Ok(t) = track {
                tracks.push(t);
            }
        }
    }
    Ok(tracks)
}

#[tauri::command]
async fn get_albums(state: State<'_, AppState>) -> Result<Vec<Album>, String> {
    let mut albums = Vec::new();
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT id, title, artist, cover_art_path FROM albums ORDER BY artist, title").map_err(|e| e.to_string())?;
        let album_iter = stmt.query_map([], |row| {
            Ok(Album {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                cover_art_path: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?;

        for album in album_iter {
            if let Ok(a) = album {
                albums.push(a);
            }
        }
    }
    Ok(albums)
}

#[tauri::command]
async fn get_album_tracks(state: State<'_, AppState>, album_id: i64) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT id, title, artist, album_id, track_number, file_path FROM tracks WHERE album_id = ?1 ORDER BY track_number").map_err(|e| e.to_string())?;
        let track_iter = stmt.query_map([&album_id], |row| {
            Ok(LocalTrack {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album_id: row.get(3)?,
                track_number: row.get(4)?,
                file_path: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        for track in track_iter {
            if let Ok(t) = track {
                tracks.push(t);
            }
        }
    }
    Ok(tracks)
}

#[tauri::command]
async fn get_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    let mut playlists = Vec::new();
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT id, name FROM playlists ORDER BY created_at").map_err(|e| e.to_string())?;
        let playlist_iter = stmt.query_map([], |row| {
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        }).map_err(|e| e.to_string())?;

        for playlist in playlist_iter {
            if let Ok(p) = playlist {
                playlists.push(p);
            }
        }
    }
    Ok(playlists)
}

#[tauri::command]
async fn create_playlist(state: State<'_, AppState>, name: String) -> Result<i64, String> {
    if let Ok(conn) = state.db_conn.lock() {
        conn.execute("INSERT INTO playlists (name) VALUES (?1)", [&name]).map_err(|e| e.to_string())?;
        return Ok(conn.last_insert_rowid());
    }
    Err("Failed to acquire lock".into())
}

#[tauri::command]
async fn add_to_playlist(state: State<'_, AppState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT COALESCE(MAX(position), 0) FROM playlist_tracks WHERE playlist_id = ?1").map_err(|e| e.to_string())?;
        let max_pos: i64 = stmt.query_row([&playlist_id], |row| row.get(0)).unwrap_or(0);
        
        conn.execute("INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)", [&playlist_id, &track_id, &(max_pos + 1)]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_playlist_tracks(state: State<'_, AppState>, playlist_id: i64) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT t.id, t.title, t.artist, t.album_id, t.track_number, t.file_path FROM tracks t JOIN playlist_tracks pt ON t.id = pt.track_id WHERE pt.playlist_id = ?1 ORDER BY pt.position").map_err(|e| e.to_string())?;
        let track_iter = stmt.query_map([&playlist_id], |row| {
            Ok(LocalTrack {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album_id: row.get(3)?,
                track_number: row.get(4)?,
                file_path: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        for track in track_iter {
            if let Ok(t) = track {
                tracks.push(t);
            }
        }
    }
    Ok(tracks)
}

#[tauri::command]
async fn remove_from_playlist(state: State<'_, AppState>, playlist_id: i64, track_id: i64) -> Result<(), String> {
    if let Ok(mut conn) = state.db_conn.lock() {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        
        let position: Option<i64> = tx.query_row(
            "SELECT position FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            [&playlist_id, &track_id],
            |row| row.get(0)
        ).ok();

        if let Some(pos) = position {
            tx.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
                [&playlist_id, &track_id]
            ).map_err(|e| e.to_string())?;

            tx.execute(
                "UPDATE playlist_tracks SET position = position - 1 WHERE playlist_id = ?1 AND position > ?2",
                [&playlist_id, &pos]
            ).map_err(|e| e.to_string())?;
        }
        
        tx.commit().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn delete_playlist(state: State<'_, AppState>, playlist_id: i64) -> Result<(), String> {
    if let Ok(conn) = state.db_conn.lock() {
        conn.execute("DELETE FROM playlist_tracks WHERE playlist_id = ?1", [&playlist_id]).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM playlists WHERE id = ?1", [&playlist_id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn rename_playlist(state: State<'_, AppState>, playlist_id: i64, new_name: String) -> Result<(), String> {
    if let Ok(conn) = state.db_conn.lock() {
        conn.execute("UPDATE playlists SET name = ?1 WHERE id = ?2", (&new_name, &playlist_id)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn reorder_playlist_track(state: State<'_, AppState>, playlist_id: i64, from_pos: i64, to_pos: i64) -> Result<(), String> {
    if from_pos == to_pos {
        return Ok(());
    }

    if let Ok(mut conn) = state.db_conn.lock() {
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE playlist_tracks SET position = -1 WHERE playlist_id = ?1 AND position = ?2",
            [&playlist_id, &from_pos]
        ).map_err(|e| e.to_string())?;

        if from_pos < to_pos {
            tx.execute(
                "UPDATE playlist_tracks SET position = position - 1 WHERE playlist_id = ?1 AND position > ?2 AND position <= ?3",
                [&playlist_id, &from_pos, &to_pos]
            ).map_err(|e| e.to_string())?;
        } else {
            tx.execute(
                "UPDATE playlist_tracks SET position = position + 1 WHERE playlist_id = ?1 AND position >= ?2 AND position < ?3",
                [&playlist_id, &to_pos, &from_pos]
            ).map_err(|e| e.to_string())?;
        }

        tx.execute(
            "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND position = -1",
            [&to_pos, &playlist_id]
        ).map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
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
        if let Ok(tag) = id3::Tag::read_from_path(&file_path) {
            if let Some(pic) = tag.pictures().next() {
                if let Ok(_) = std::fs::write(&dest_path, &pic.data) {
                    return Ok(Some(dest_path.to_string_lossy().to_string()));
                }
            }
        }
        Ok(None)
    }).await.map_err(|e| e.to_string())?;
    
    Ok(result?)
}

#[tauri::command]
async fn clear_local_library(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(conn) = state.db_conn.lock() {
        conn.execute("DELETE FROM tracks", []).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1").map_err(|e| e.to_string())?;
        let mut rows = stmt.query([&key]).map_err(|e| e.to_string())?;
        if let Ok(Some(row)) = rows.next() {
            let value: String = row.get(0).map_err(|e| e.to_string())?;
            return Ok(Some(value));
        }
    }
    Ok(None)
}

#[tauri::command]
async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    if let Ok(conn) = state.db_conn.lock() {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            (&key, &value),
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn factory_reset(app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // 1. Clear database completely
    if let Ok(conn) = state.db_conn.lock() {
        let _ = conn.execute("DELETE FROM tracks", []);
        let _ = conn.execute("DELETE FROM settings", []);
    }

    // 2. Remove providers dir
    let app_data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let providers_dir = app_data_dir.join("providers");
    let _ = std::fs::remove_dir_all(&providers_dir);

    Ok(())
}

#[tauri::command]
async fn set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::SetVolume(volume));
    }
    Ok(())
}

#[tauri::command]
async fn set_mute(state: State<'_, AppState>, mute: bool) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::SetMute(mute));
    }
    Ok(())
}

// 5. Async Tauri Commands
#[tauri::command]
async fn load_audio(state: State<'_, AppState>, path: String) -> Result<(), String> {
    // Cache the track in SQLite when loaded
    if let Ok(conn) = state.db_conn.lock() {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO tracks (title, file_path) VALUES (?1, ?2)",
            ("Unknown Title", &path),
        );
    }

    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Load(path));
    }
    Ok(())
}

#[tauri::command]
async fn play_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Play);
    }
    Ok(())
}

#[tauri::command]
async fn pause_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Pause);
    }
    Ok(())
}

#[tauri::command]
async fn stop_audio(state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(tx) = state.tx.lock() {
        let _ = tx.send(AudioCommand::Stop);
    }
    Ok(())
}

#[tauri::command]
fn seek_audio(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    let tx = state.tx.lock().map_err(|e| e.to_string())?;
    tx.send(AudioCommand::Seek(position)).map_err(|e| e.to_string())
}

// 6. App Initialization
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (tx, rx) = mpsc::channel();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let handle = app.handle().clone();
            
            // 1. Resolve OS directories
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let db_dir = app_data_dir.join("database");
            let providers_dir = app_data_dir.join("providers");
            
            std::fs::create_dir_all(&db_dir).expect("Failed to create database directory");
            std::fs::create_dir_all(&providers_dir).expect("Failed to create providers directory");
            
            // 2. Initialize Database
            let db_path = db_dir.join("echo_library.db");
            let db = init_db(&db_path).expect("Failed to initialize SQLite");
            
            // 3. Pre-populate default providers if the directory is empty
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

            // 4. Manage State & Start Background Threads
            app.manage(AppState {
                tx: Mutex::new(tx),
                db_conn: Mutex::new(db),
            });
            
            start_audio_thread(rx, handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_audio,
            play_audio,
            pause_audio,
            stop_audio,
            seek_audio,
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
            extract_and_cache_artwork,
            clear_local_library,
            get_setting,
            set_setting,
            factory_reset,
            set_volume,
            set_mute
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
