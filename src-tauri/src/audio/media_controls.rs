use souvlaki::{MediaControls, MediaMetadata, PlatformConfig, MediaPlayback};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Emitter};

pub struct OSMediaControls {
    controls: Mutex<Option<MediaControls>>,
}

impl OSMediaControls {
    pub fn new(app_handle: &AppHandle) -> Self {
        #[cfg(target_os = "windows")]
        let hwnd = {
            use tauri::Manager;
            if let Some(window) = app_handle.get_webview_window("main") {
                if let Ok(hwnd) = window.hwnd() {
                    Some(hwnd.0 as *mut std::ffi::c_void)
                } else {
                    None
                }
            } else {
                None
            }
        };

        #[cfg(not(target_os = "windows"))]
        let hwnd = None;

        let config = PlatformConfig {
            dbus_name: "echo_desktop",
            display_name: "Echo Music Player",
            hwnd,
        };

        let mut controls = MediaControls::new(config);
        
        if let Ok(ref mut controls) = controls {
            let app_handle_clone = app_handle.clone();
            controls.attach(move |event| {
                use souvlaki::MediaControlEvent::*;
                match event {
                    Play => {
                        let tx = {
                            let state = app_handle_clone.state::<crate::AppState>();
                            state.audio_tx.lock().ok().map(|l| l.clone())
                        };
                        if let Some(tx) = tx {
                            let _ = tx.send(crate::audio::AudioCommand::Play);
                        }
                    }
                    Pause => {
                        let tx = {
                            let state = app_handle_clone.state::<crate::AppState>();
                            state.audio_tx.lock().ok().map(|l| l.clone())
                        };
                        if let Some(tx) = tx {
                            let _ = tx.send(crate::audio::AudioCommand::Pause);
                        }
                    }
                    Toggle => {
                        let _ = app_handle_clone.emit("media-play-pause", ());
                    }
                    Next => {
                        let _ = app_handle_clone.emit("media-next", ());
                    }
                    Previous => {
                        let _ = app_handle_clone.emit("media-prev", ());
                    }
                    Stop => {
                        let tx = {
                            let state = app_handle_clone.state::<crate::AppState>();
                            state.audio_tx.lock().ok().map(|l| l.clone())
                        };
                        if let Some(tx) = tx {
                            let _ = tx.send(crate::audio::AudioCommand::Stop);
                        }
                    }
                    _ => {}
                }
            }).ok();
        }

        Self {
            controls: Mutex::new(controls.ok()),
        }
    }

    pub fn update_metadata(&self, title: &str, artist: &str, album: &str, duration: Option<std::time::Duration>) {
        if let Ok(mut lock) = self.controls.lock() {
            if let Some(controls) = lock.as_mut() {
                let mut metadata = MediaMetadata {
                    title: Some(title),
                    artist: Some(artist),
                    album: Some(album),
                    ..Default::default()
                };
                if let Some(d) = duration {
                    metadata.duration = Some(d);
                }
                let _ = controls.set_metadata(metadata);
            }
        }
    }

    pub fn set_playback_status(&self, playing: bool) {
        if let Ok(mut lock) = self.controls.lock() {
            if let Some(controls) = lock.as_mut() {
                let status = if playing {
                    MediaPlayback::Playing { progress: None }
                } else {
                    MediaPlayback::Paused { progress: None }
                };
                let _ = controls.set_playback(status);
            }
        }
    }
}
