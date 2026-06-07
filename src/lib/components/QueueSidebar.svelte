<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    import { X, Trash2 } from "lucide-svelte";
    import { fly, fade } from "svelte/transition";

    let { open = $bindable(false) } = $props<{ open?: boolean }>();

    function jumpToTrack(index: number) {
        audioStore.queueIndex = index;
        audioStore.load(audioStore.queue[index].file_path);
    }

    function isCurrentTrack(index: number): boolean {
        return index === audioStore.queueIndex;
    }
</script>

{#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="queue-backdrop" transition:fade={{ duration: 200 }} onclick={() => open = false}></div>
    <aside class="queue-sidebar" transition:fly={{ x: 350, duration: 250 }}>
        <div class="queue-header">
            <h3>Queue</h3>
            <div style="display: flex; gap: 0.5rem; align-items: center;">
                {#if audioStore.queue.length > 0}
                    <button class="ghost icon-btn" onclick={() => audioStore.clearQueue()} title="Clear Queue">
                        <Trash2 size={16} />
                    </button>
                {/if}
                <button class="ghost icon-btn" onclick={() => open = false} title="Close">
                    <X size={18} />
                </button>
            </div>
        </div>

        {#if audioStore.queue.length === 0}
            <div class="queue-empty">
                <p class="text-muted">No tracks in queue.</p>
                <p class="text-muted" style="font-size: 0.8rem; margin-top: 0.5rem;">
                    Play an album or playlist to populate the queue.
                </p>
            </div>
        {:else}
            <div class="queue-list">
                {#each audioStore.queue as track, i}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div 
                        class="queue-item" 
                        class:active={isCurrentTrack(i)}
                        onclick={() => jumpToTrack(i)}
                    >
                        <div class="queue-item-left">
                            <span class="queue-num">{i + 1}</span>
                            <div class="queue-item-info">
                                <span class="queue-item-title">{track.title}</span>
                                <span class="queue-item-artist text-muted">{track.artist || "Unknown Artist"}</span>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </aside>
{/if}

<style>
    .queue-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.3);
        z-index: 40;
    }

    .queue-sidebar {
        position: fixed;
        top: 0;
        right: 0;
        width: 350px;
        height: calc(100vh - 100px); /* Stop above PlayerBar */
        background: rgba(11, 12, 16, 0.92);
        backdrop-filter: blur(24px);
        -webkit-backdrop-filter: blur(24px);
        border-left: 1px solid var(--glass-border);
        z-index: 50;
        display: flex;
        flex-direction: column;
    }

    .queue-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.5rem;
        border-bottom: 1px solid var(--glass-border);
        flex-shrink: 0;
    }

    .queue-header h3 {
        font-family: var(--font-display);
        font-weight: 600;
        color: #fff;
        margin: 0;
    }

    .icon-btn {
        padding: 0.35rem;
        border-radius: 6px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .queue-empty {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 2rem;
        text-align: center;
    }

    .queue-list {
        flex: 1;
        overflow-y: auto;
        padding: 0.5rem 0;
    }

    .queue-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.6rem 1.5rem;
        cursor: pointer;
        transition: background 0.15s;
        border-left: 3px solid transparent;
    }

    .queue-item:hover {
        background: rgba(255, 255, 255, 0.05);
    }

    .queue-item.active {
        background: rgba(102, 252, 241, 0.08);
        border-left-color: var(--color-cyan);
    }

    .queue-item-left {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        min-width: 0;
        flex: 1;
    }

    .now-indicator {
        color: var(--color-cyan);
        display: flex;
        align-items: center;
        justify-content: center;
        width: 20px;
        flex-shrink: 0;
    }

    .queue-num {
        color: var(--color-chalk-muted);
        font-size: 0.8rem;
        width: 20px;
        text-align: right;
        flex-shrink: 0;
    }

    .queue-item-info {
        display: flex;
        flex-direction: column;
        min-width: 0;
    }

    .queue-item-title {
        font-size: 0.9rem;
        color: #fff;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .queue-item.active .queue-item-title {
        color: var(--color-cyan);
    }

    .queue-item-artist {
        font-size: 0.8rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
</style>
