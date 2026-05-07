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
pub struct LocalTrack {
    pub id: i64,
    pub title: String,
    pub file_path: String,
}

// 1. Message Payload
pub enum AudioCommand {
    Load(String),
    Play,
    Pause,
    Stop,
}

// 2. Global State
struct AppState {
    tx: Mutex<Sender<AudioCommand>>,
    db_conn: Mutex<Connection>,
}

// 3. Database Initialization
fn init_db<P: AsRef<std::path::Path>>(db_path: P) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            file_path TEXT UNIQUE NOT NULL
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

// 4. The Audio Thread (Now with Event Emission)
fn start_audio_thread(rx: Receiver<AudioCommand>, app_handle: AppHandle) {
    thread::spawn(move || {
        let (_stream, stream_handle) =
            OutputStream::try_default().expect("Failed to get audio output");
        let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");

        loop {
            if let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::Load(path) => {
                        sink.stop();
                        if let Ok(file) = File::open(&path) {
                            let reader = BufReader::new(file);
                            if let Ok(decoder) = Decoder::new(reader) {
                                sink.append(decoder);
                                sink.play();
                                // Emit state back to Svelte
                                let _ = app_handle.emit("player-state", "Playing");
                                let _ = app_handle.emit("current-track", path.clone());
                            }
                        }
                    }
                    AudioCommand::Play => {
                        sink.play();
                        let _ = app_handle.emit("player-state", "Playing");
                    }
                    AudioCommand::Pause => {
                        sink.pause();
                        let _ = app_handle.emit("player-state", "Paused");
                    }
                    AudioCommand::Stop => {
                        sink.stop();
                        let _ = app_handle.emit("player-state", "Stopped");
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
                        
                        // Try to extract ID3 title
                        if let Ok(tag) = id3::Tag::read_from_path(entry_path) {
                            if let Some(tag_title) = tag.title() {
                                title = tag_title.to_string();
                            }
                        }
                        
                        found_tracks.push((title, entry_path.to_string_lossy().to_string()));
                    }
                }
            }
        }
        found_tracks
    }).await.map_err(|e| e.to_string())?;

    let mut added = 0;
    // Keep the mutex lock extremely brief by just doing a batch insert
    if let Ok(conn) = state.db_conn.lock() {
        for (title, file_path) in tracks {
            let res = conn.execute(
                "INSERT OR IGNORE INTO tracks (title, file_path) VALUES (?1, ?2)",
                (&title, &file_path),
            );
            if res.unwrap_or(0) > 0 {
                added += 1;
            }
        }
    }
    
    Ok(added)
}

#[tauri::command]
async fn get_local_tracks(state: State<'_, AppState>) -> Result<Vec<LocalTrack>, String> {
    let mut tracks = Vec::new();
    if let Ok(conn) = state.db_conn.lock() {
        let mut stmt = conn.prepare("SELECT id, title, file_path FROM tracks").map_err(|e| e.to_string())?;
        let track_iter = stmt.query_map([], |row| {
            Ok(LocalTrack {
                id: row.get(0)?,
                title: row.get(1)?,
                file_path: row.get(2)?,
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

// 6. App Initialization
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (tx, rx) = mpsc::channel();

    tauri::Builder::default()
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
            search_provider,
            scan_local_directory,
            get_local_tracks,
            clear_local_library,
            get_setting,
            set_setting,
            factory_reset
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
