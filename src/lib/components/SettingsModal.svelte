<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { libraryStore } from "$lib/stores/library.svelte";

    let { onClose } = $props<{ onClose: () => void }>();

    async function factoryReset() {
        if (confirm("Are you sure you want to completely wipe your library and settings?")) {
            try {
                await invoke("factory_reset");
                libraryStore.albums = [];
                libraryStore.playlists = [];
                alert("Library wiped. Please restart the app or scan again.");
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
        
        <div style="border-top: 1px solid var(--glass-border); padding-top: 1.5rem;">
            <label style="display: block; margin-bottom: 1rem; color: var(--color-chalk-muted);">Data Management</label>
            <p style="margin-bottom: 1rem; font-size: 0.9rem;">
                This will delete the local SQLite database and clear all cached artwork and extensions. Your actual music files will not be touched.
            </p>
            <button style="border-color: var(--color-danger); color: var(--color-danger); width: 100%;" onclick={factoryReset}>
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
    }
</style>
