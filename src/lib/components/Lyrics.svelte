<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    
    // Scaffolding for lyrics synced with audioStore
    // In the future, this component will take the current_track
    // and fetch synced LRC from network provider extensions.
</script>

<div class="lyrics-container">
    <div class="placeholder">
        <p class="text-2">Lyrics</p>
        <p class="text-3" style="font-size: 0.875rem; margin-top: 0.5rem;">Syncing coming soon...</p>
    </div>
</div>

<style>
    .lyrics-container {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
        background: var(--echo-surface);
        border-radius: 1.5rem;
    }
    
    .placeholder {
        text-align: center;
    }
</style>
