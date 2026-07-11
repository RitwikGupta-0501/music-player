<script lang="ts">
    import { audioStore } from "$lib/stores/audio.svelte";
    // @ts-ignore - svelte-virtual-list lacks type definitions
    import VirtualList from "svelte-virtual-list";
    import { X, Trash, Pause, Play } from "phosphor-svelte";
    let { open = $bindable(false) } = $props<{ open?: boolean }>();

    let draggedIndex = -1;
    let dragoverIndex = $state(-1);
    let justReorderedIndex = $state(-1);
    let isLoading = $state(false);

    let queueWithIndex = $derived(
        audioStore.queue.map((track, i) => ({ track, index: i }))
    );

    function jumpToTrack(instanceId: string) {
        audioStore.jumpToTrack(instanceId);
    }

    async function handleDragStart(e: DragEvent, index: number) {
        draggedIndex = index;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = "move";
            const track = audioStore.queue[index];
            const dragImage = createDragPreview(track.title);
            e.dataTransfer.setDragImage(dragImage, 0, 0);
        }
    }

    async function handleDrop(e: DragEvent, targetIndex: number) {
        e.preventDefault();
        dragoverIndex = -1;

        if (draggedIndex !== -1 && draggedIndex !== targetIndex) {
            isLoading = true;
            try {
                await audioStore.reorderQueue(draggedIndex, targetIndex);
                justReorderedIndex = targetIndex;
                setTimeout(() => {
                    justReorderedIndex = -1;
                }, 300);
            } catch (error) {
                console.error("Reorder failed:", error);
            } finally {
                isLoading = false;
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

    function handleDragLeave() {
        dragoverIndex = -1;
    }

    function createDragPreview(title: string): HTMLElement {
        const div = document.createElement("div");
        div.style.position = "absolute";
        div.style.top = "-9999px";
        div.style.left = "-9999px";
        div.style.background = "rgba(255, 255, 255, 0.1)";
        div.style.border = "1px solid rgba(255, 255, 255, 0.2)";
        div.style.borderRadius = "6px";
        div.style.padding = "0.6rem 1rem";
        div.style.fontSize = "0.8rem";
        div.style.color = "rgb(200, 200, 200)";
        div.style.whiteSpace = "nowrap";
        div.textContent = title;
        document.body.appendChild(div);
        setTimeout(() => document.body.removeChild(div), 0);
        return div;
    }

    async function handleClearQueue() {
        if (confirm("Clear entire queue?")) {
            isLoading = true;
            try {
                await audioStore.clearQueue();
            } finally {
                isLoading = false;
            }
        }
    }
</script>

{#if open}
    <div class="queue-view">
        <div class="queue-header">
            <span class="queue-title">Up Next</span>
            <div class="header-actions">
                {#if audioStore.queue.length > 0}
                    <button
                        class="icon-btn"
                        onclick={handleClearQueue}
                        disabled={isLoading}
                        title="Clear queue"
                    >
                        <Trash size={16} weight="bold" />
                    </button>
                {/if}
            </div>
        </div>

        {#if audioStore.queue.length === 0}
            <div class="queue-empty">
                <p class="empty-label">Queue is empty</p>
                <p class="empty-hint">Play an album or playlist to populate it.</p>
            </div>
        {:else}
            <VirtualList items={queueWithIndex} let:item={row}>
                {@const track = row.track}
                {@const i = row.index}
                {@const isPlaying = track.instanceId === audioStore.currentQueueId}

                <div
                    class="queue-row"
                    class:playing={isPlaying}
                    class:drag-over={dragoverIndex === i}
                    class:just-reordered={justReorderedIndex === i}
                    role="button"
                    tabindex="0"
                    draggable="true"
                    ondragstart={(e) => handleDragStart(e, i)}
                    ondragover={(e) => handleDragOver(e, i)}
                    ondragleave={handleDragLeave}
                    ondrop={(e) => handleDrop(e, i)}
                    ondragend={() => {
                        dragoverIndex = -1;
                        draggedIndex = -1;
                    }}
                    onclick={() => jumpToTrack(track.instanceId)}
                    onkeydown={(e) => e.key === 'Enter' && jumpToTrack(track.instanceId)}
                >
                    <span class="row-num">
                        {#if isPlaying}
                            <span class="playing-indicator">
                                {#if audioStore.playbackState === "Playing"}
                                    <Pause size={20} weight="fill" color="#b58e62" />
                                {:else}
                                    <Play size={20} weight="fill" color="#b58e62" />
                                {/if}
                            </span>
                        {:else}
                            {i + 1}
                        {/if}
                    </span>
                    <div class="row-info">
                        <span class="row-title">{track.title}</span>
                        <span class="row-artist">{track.artist || "Unknown"}</span>
                    </div>
                </div>
            </VirtualList>
        {/if}
    </div>
{/if}

<style>
    .queue-view {
        display: flex;
        flex-direction: column;
        height: 100%;
        width: 100%;
        padding-bottom: 5rem; /* Allow space for player bar */
    }

    .queue-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1.25rem;
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
        transition: all 0.12s ease;
    }

    .icon-btn:hover:not(:disabled) {
        color: var(--echo-text-1);
        background: rgba(255 255 255 / 0.07);
    }

    .icon-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .icon-btn:active:not(:disabled) {
        transform: scale(0.94);
    }

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

    :global(.virtual-list-container) {
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
        0% {
            background: rgba(255, 255, 255, 0.12);
            transform: scale(1);
        }
        50% {
            background: rgba(255, 255, 255, 0.08);
        }
        100% {
            background: rgba(255, 255, 255, 0.04);
            transform: scale(1);
        }
    }

    .row-num {
        font-size: 0.72rem;
        color: var(--echo-text-3);
        width: 24px;
        text-align: center;
        flex-shrink: 0;
        font-variant-numeric: tabular-nums;
    }

    .playing-indicator {
        color: var(--echo-silver);
        font-weight: bold;
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
</style>
