<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    import { X, Trash2 } from "lucide-svelte";
    import { fly, fade } from "svelte/transition";
    import { cubicOut } from "svelte/easing";

    let { open = $bindable(false) } = $props<{ open?: boolean }>();

    let draggedIndex = -1;
    let dragoverIndex = $state(-1);
    let justReorderedIndex = $state(-1);

    function jumpToTrack(instanceId: string) {
        audioStore.jumpToTrack(instanceId);
    }

    function handleDragStart(e: DragEvent, index: number) {
        draggedIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = "move";
            // Create a custom drag image
            const dragImage = document.createElement('div');
            dragImage.style.position = 'absolute';
            dragImage.style.top = '-9999px';
            dragImage.style.left = '-9999px';
            dragImage.style.background = 'rgba(255, 255, 255, 0.1)';
            dragImage.style.border = '1px solid rgba(255, 255, 255, 0.2)';
            dragImage.style.borderRadius = '6px';
            dragImage.style.padding = '0.6rem 1rem';
            dragImage.style.fontSize = '0.8rem';
            dragImage.style.color = 'rgb(200, 200, 200)';
            dragImage.style.whiteSpace = 'nowrap';
            dragImage.textContent = audioStore.queue[index]?.title || 'Track';
            document.body.appendChild(dragImage);
            e.dataTransfer.setDragImage(dragImage, 0, 0);
            setTimeout(() => document.body.removeChild(dragImage), 0);
        }
    }

    async function handleDrop(e: DragEvent, targetIndex: number) {
        e.preventDefault();
        dragoverIndex = -1;
        if (draggedIndex !== -1 && draggedIndex !== targetIndex) {
            try {
                await audioStore.reorderQueue(draggedIndex, targetIndex);
                justReorderedIndex = targetIndex;
                setTimeout(() => {
                    justReorderedIndex = -1;
                }, 300);
            } catch (error) {
                console.error("Failed to reorder queue:", error);
            }
        }
        draggedIndex = -1;
    }

    function handleDragOver(e: DragEvent, index: number) {
        e.preventDefault();
        dragoverIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.dropEffect = "move";
        }
    }

    function handleDragLeave(e: DragEvent) {
        // Only clear if leaving the row entirely
        const target = e.relatedTarget as HTMLElement;
        if (!target?.closest('.queue-row')) {
            dragoverIndex = -1;
        }
    }
</script>

{#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="backdrop"
        transition:fade={{ duration: 180 }}
        onclick={() => (open = false)}
    ></div>

    <aside
        class="queue-panel"
        transition:fly={{ x: 360, duration: 260, easing: cubicOut }}
    >
        <div class="queue-header">
            <span class="queue-title">Queue</span>
            <div class="header-actions">
                {#if audioStore.queue.length > 0}
                    <button
                        class="icon-btn"
                        onclick={() => audioStore.clearQueue()}
                        title="Clear queue"
                    >
                        <Trash2 size={14} />
                    </button>
                {/if}
                <button class="icon-btn" onclick={() => (open = false)} title="Close">
                    <X size={16} />
                </button>
            </div>
        </div>

        {#if audioStore.queue.length === 0}
            <div class="queue-empty">
                <p class="empty-label">Queue is empty</p>
                <p class="empty-hint">Play an album or playlist to populate it.</p>
            </div>
        {:else}
            <div class="queue-list">
                {#each audioStore.queue as track, i (track.instanceId)}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div
                        class="queue-row"
                        class:playing={track.instanceId === audioStore.currentQueueId}
                        class:drag-over={dragoverIndex === i}
                        class:just-reordered={justReorderedIndex === i}
                        draggable="true"
                        ondragstart={(e) => handleDragStart(e, i)}
                        ondragenter={(e) => { e.preventDefault(); dragoverIndex = i; }}
                        ondragover={(e) => handleDragOver(e, i)}
                        ondragleave={handleDragLeave}
                        ondrop={(e) => handleDrop(e, i)}
                        ondragend={() => { dragoverIndex = -1; draggedIndex = -1; }}
                        onclick={() => jumpToTrack(track.instanceId)}
                    >
                        <span class="row-num">
                            {#if track.instanceId === audioStore.currentQueueId}
                                <!-- Playing indicator: three animated bars -->
                                <span class="playing-bars" aria-label="Now playing">
                                    <span></span>
                                    <span></span>
                                    <span></span>
                                </span>
                            {:else}
                                {i + 1}
                            {/if}
                        </span>
                        <div class="row-info">
                            <span class="row-title">{track.title}</span>
                            <span class="row-artist">{track.artist || "Unknown Artist"}</span>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </aside>
{/if}

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0 0 0 / 0.25);
        z-index: 45;
    }

    .queue-panel {
        position: fixed;
        top: 0;
        right: 0;
        /* Stop just above the floating player bar (~120px from bottom) */
        height: calc(100vh - 120px);
        width: 320px;
        background: rgba(9 9 12 / 0.88);
        backdrop-filter: blur(44px) saturate(180%);
        -webkit-backdrop-filter: blur(44px) saturate(180%);
        border-left: 1px solid var(--echo-border);
        box-shadow: -20px 0 60px rgba(0 0 0 / 0.45);
        z-index: 55;
        display: flex;
        flex-direction: column;
    }

    @media (prefers-reduced-transparency: reduce) {
        .queue-panel {
            background: var(--echo-base);
            backdrop-filter: none;
            -webkit-backdrop-filter: none;
        }
    }

    /* ── Header ── */
    .queue-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.25rem 1.25rem 1rem;
        border-bottom: 1px solid var(--echo-border);
        flex-shrink: 0;
    }

    .queue-title {
        font-size: 0.8rem;
        font-weight: 600;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: var(--echo-text-2);
    }

    .header-actions {
        display: flex;
        gap: 4px;
        align-items: center;
    }

    .icon-btn {
        background: transparent;
        border: none;
        color: var(--echo-text-3);
        padding: 0.3rem;
        border-radius: 6px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: color 0.12s ease, background 0.12s ease;
    }
    .icon-btn:hover {
        color: var(--echo-text-1);
        background: rgba(255 255 255 / 0.07);
    }
    .icon-btn:active { transform: scale(0.94); }

    /* ── Empty state ── */
    .queue-empty {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 0.4rem;
        padding: 2rem;
        text-align: center;
    }

    .empty-label {
        font-size: 0.85rem;
        color: var(--echo-text-2);
        font-weight: 500;
    }

    .empty-hint {
        font-size: 0.75rem;
        color: var(--echo-text-3);
        max-width: 200px;
        line-height: 1.5;
    }

    /* ── Track list ── */
    .queue-list {
        flex: 1;
        overflow-y: auto;
        padding: 0.375rem 0;
    }

    .queue-row {
        position: relative;
        display: flex;
        align-items: center;
        gap: 0.85rem;
        padding: 0.6rem 1.25rem;
        cursor: pointer;
        transition: background 0.15s ease, opacity 0.15s ease, transform 0.15s ease;
        user-select: none;
    }

    .queue-row:hover {
        background: rgba(255 255 255 / 0.04);
    }

    .queue-row.drag-over {
        border-top: 2px solid var(--echo-accent);
        background: rgba(255 255 255 / 0.05);
    }

    .queue-row.playing {
        background: rgba(255 255 255 / 0.04);
        border-left-color: var(--echo-silver);
    }

    .queue-row[draggable="true"]:active {
        opacity: 0.6;
        transform: scale(0.98);
    }

    .queue-row.just-reordered {
        background: rgba(255, 255, 255, 0.08);
        animation: pulse-highlight 0.3s ease-out;
    }

    @keyframes pulse-highlight {
        0% { background: rgba(255, 255, 255, 0.12); transform: scale(1); }
        50% { background: rgba(255, 255, 255, 0.08); }
        100% { background: rgba(255, 255, 255, 0.04); transform: scale(1); }
    }

    .row-num {
        font-size: 0.72rem;
        color: var(--echo-text-3);
        width: 18px;
        text-align: right;
        flex-shrink: 0;
        font-variant-numeric: tabular-nums;
    }

    .row-info {
        min-width: 0;
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .row-title {
        font-size: 0.8rem;
        font-weight: 450;
        color: var(--echo-text-1);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .queue-row.playing .row-title {
        color: var(--echo-silver);
    }

    .row-artist {
        font-size: 0.7rem;
        color: var(--echo-text-3);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    /* ── Animated playing bars ── */
    .playing-bars {
        display: inline-flex;
        align-items: flex-end;
        gap: 2px;
        height: 12px;
    }

    .playing-bars span {
        display: block;
        width: 2px;
        background: var(--echo-silver);
        border-radius: 1px;
        animation: bar-bounce 0.9s ease-in-out infinite alternate;
    }

    .playing-bars span:nth-child(1) { height: 8px; animation-delay: 0s; }
    .playing-bars span:nth-child(2) { height: 12px; animation-delay: 0.2s; }
    .playing-bars span:nth-child(3) { height: 6px; animation-delay: 0.4s; }

    @keyframes bar-bounce {
        from { transform: scaleY(0.4); }
        to   { transform: scaleY(1); }
    }

    @media (prefers-reduced-motion: reduce) {
        .playing-bars span { animation: none; }
    }
</style>
