<script lang="ts">
    import type { Snippet } from "svelte";

    let { activeView = $bindable("albums"), actions } = $props<{ 
        activeView: string, 
        actions?: Snippet 
    }>();
</script>

<header class="lib-header">
    <div class="header-content">
        <h1 class="lib-heading font-headline-lg text-text-main leading-tight mb-2">Your Library</h1>
        <div class="tabs">
            <button class="tab" class:active={activeView === "albums"} onclick={() => activeView = "albums"}>Albums</button>
            <button class="tab" class:active={activeView === "playlists"} onclick={() => activeView = "playlists"}>Playlists</button>
        </div>
    </div>

    <div class="lib-actions">
        {@render actions?.()}
    </div>
</header>

<style>
    /* ── Library header ── */
    .lib-header {
        position: sticky;
        top: 0;
        background: rgba(5, 5, 7, 0.9);
        backdrop-filter: blur(24px);
        z-index: 20;
        padding: 3rem 2.5rem 2rem; /* pt-12 px-10 pb-8 */
        border-bottom: 1px solid rgba(255, 255, 255, 0.05); /* border-white/5 */
        margin-bottom: 0; /* Removing margin-bottom, grid has padding */
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
    }

    .header-content {
        display: flex;
        flex-direction: column;
        gap: 1.5rem; /* mt-6 for tabs */
    }

    .lib-heading {
        font-size: 2.25rem; /* text-4xl */
        margin: 0;
    }

    .tabs {
        display: flex;
        gap: 0.5rem;
        font-size: 0.875rem; /* text-sm */
        letter-spacing: 0.1em; /* tracking-widest */
        text-transform: uppercase;
        font-weight: 500;
        color: var(--echo-text-2); /* text-muted */
    }

    .tab {
        background: transparent;
        border: none;
        color: inherit;
        cursor: pointer;
        padding: 0.5rem 1rem;
        border-radius: 8px;
        transition: all 0.2s;
    }

    .tab:hover:not(.active) {
        color: var(--echo-text-1);
        background: rgba(255, 255, 255, 0.04);
    }

    .tab.active {
        color: var(--echo-primary);
        background: rgba(226, 169, 115, 0.1);
    }

    .lib-actions {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding-bottom: 0.5rem;
    }

    /* Spinner keyframe */
    :global(.spin) {
        animation: -global-spin 1s linear infinite;
    }

    @keyframes -global-spin {
        from { transform: rotate(0deg); }
        to   { transform: rotate(360deg); }
    }
</style>
