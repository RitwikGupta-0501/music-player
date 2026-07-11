<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { libraryStore } from "$lib/stores/library.svelte";

    let { onClose } = $props<{ onClose: () => void }>();

    async function factoryReset() {
        // Tauri automatically intercepts native confirm/alert when the dialog plugin is active on the backend
        const yes = confirm("Are you sure you want to completely wipe your library and settings?");
        if (yes) {
            try {
                await invoke("factory_reset");
                libraryStore.albums = [];
                libraryStore.playlists = [];
                onClose();
            } catch (e) {
                console.error("Factory reset failed:", e);
                alert("Failed to reset library.");
            }
        }
    }
</script>

<div class="modal-overlay">
    <div class="glass-panel modal-content">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem;">
            <h2>Settings</h2>
            <button class="ghost" style="padding: 0.2rem 0.5rem;" onclick={onClose}>✕</button>
        </div>
        
        <div style="border-top: 1px solid var(--echo-border); padding-top: 1.5rem; margin-top: 1.5rem;">
            <h3 style="display: block; margin-bottom: 1rem; color: var(--echo-text-2); font-size: 1rem; font-weight: normal; font-family: var(--echo-font-body);">Audio & Playback</h3>
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                <div>
                    <p style="font-size: 0.95rem; color: var(--echo-text-1);">Keep Playing on Queue Clear</p>
                    <p style="font-size: 0.8rem; color: var(--echo-text-2); margin-top: 0.2rem;">Allow the current song to finish even if the upcoming queue is wiped.</p>
                </div>
                <label class="switch">
                    <input type="checkbox" checked />
                    <span class="slider round"></span>
                </label>
            </div>
        </div>

        <div style="border-top: 1px solid var(--echo-border); padding-top: 1.5rem; margin-top: 1.5rem;">
            <h3 style="display: block; margin-bottom: 1rem; color: var(--echo-text-2); font-size: 1rem; font-weight: normal; font-family: var(--echo-font-body);">Data Management</h3>
            <p style="margin-bottom: 1rem; font-size: 0.9rem; color: var(--echo-text-2);">
                This will delete the local SQLite database and clear all cached artwork and extensions. Your actual music files will not be touched.
            </p>
            <button class="primary" style="background-color: rgba(220, 38, 38, 0.2); color: #ef4444; border: 1px solid rgba(220, 38, 38, 0.4); width: 100%;" onclick={factoryReset}>
                Factory Reset (Wipe Database)
            </button>
        </div>
    </div>
</div>

<style>
    .modal-overlay {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        background: rgba(0,0,0,0.7);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 100;
        backdrop-filter: blur(4px);
    }
    .modal-content {
        padding: 2.5rem;
        width: 100%;
        max-width: 500px;
        display: flex;
        flex-direction: column;
        background: var(--echo-surface);
        border: 1px solid var(--echo-border);
        border-radius: 1.5rem;
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
    }

    /* Switch CSS */
    .switch {
        position: relative;
        display: inline-block;
        width: 44px;
        height: 24px;
    }
    .switch input {
        opacity: 0;
        width: 0;
        height: 0;
    }
    .slider {
        position: absolute;
        cursor: pointer;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: var(--echo-raised);
        border: 1px solid var(--echo-border-medium);
        transition: .4s;
    }
    .slider:before {
        position: absolute;
        content: "";
        height: 16px;
        width: 16px;
        left: 3px;
        bottom: 3px;
        background-color: var(--echo-text-2);
        transition: .4s;
    }
    input:checked + .slider {
        background-color: var(--echo-primary);
        border-color: var(--echo-primary);
    }
    input:checked + .slider:before {
        transform: translateX(20px);
        background-color: var(--echo-void);
    }
    .slider.round {
        border-radius: 34px;
    }
    .slider.round:before {
        border-radius: 50%;
    }
</style>
