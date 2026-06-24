<script lang="ts">
    import { Disc3, ListMusic, Radio, Settings2 } from "lucide-svelte";
    let { activeView = $bindable("albums") } = $props<{ activeView?: string }>();
</script>

<aside class="sidebar">
    <!-- Wordmark -->
    <div class="sidebar-top">
        <span class="wordmark">echo</span>
    </div>

    <!-- Primary navigation -->
    <nav class="sidebar-nav">
        <button
            class="nav-item"
            class:active={activeView === "albums"}
            onclick={() => activeView = "albums"}
        >
            <Disc3 size={16} strokeWidth={1.5} />
            <span>Library</span>
        </button>
        <button
            class="nav-item"
            class:active={activeView === "playlists"}
            onclick={() => activeView = "playlists"}
        >
            <ListMusic size={16} strokeWidth={1.5} />
            <span>Playlists</span>
        </button>
        <button
            class="nav-item"
            class:active={activeView === "providers"}
            onclick={() => activeView = "providers"}
        >
            <Radio size={16} strokeWidth={1.5} />
            <span>Network</span>
        </button>
    </nav>

    <!-- Settings pinned to bottom -->
    <div class="sidebar-footer">
        <button
            class="nav-item"
            class:active={activeView === "settings"}
            onclick={() => activeView = "settings"}
        >
            <Settings2 size={16} strokeWidth={1.5} />
            <span>Settings</span>
        </button>
    </div>
</aside>

<style>
    .sidebar {
        background: rgba(0 0 0 / 0.45);
        backdrop-filter: blur(20px) saturate(160%);
        -webkit-backdrop-filter: blur(20px) saturate(160%);
        border-right: 1px solid var(--echo-border);
        height: 100vh;
        display: flex;
        flex-direction: column;
        padding: 1.75rem 0 1.5rem;
        position: sticky;
        top: 0;
        user-select: none;
    }

    /* Reduced-transparency fallback */
    @media (prefers-reduced-transparency: reduce) {
        .sidebar {
            background: var(--echo-surface);
            backdrop-filter: none;
            -webkit-backdrop-filter: none;
        }
    }

    .sidebar-top {
        padding: 0 1.5rem 2rem;
    }

    .wordmark {
        font-family: var(--echo-font);
        font-size: 1.05rem;
        font-weight: 700;
        letter-spacing: -0.05em;
        color: var(--echo-text-1);
    }

    .sidebar-nav {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 0.7rem;
        padding: 0.6rem 1.25rem 0.6rem 1.375rem;
        background: transparent;
        border: none;
        border-left: 2px solid transparent;
        border-radius: 0;
        color: var(--echo-text-3);
        font-size: 0.825rem;
        font-weight: 450;
        cursor: pointer;
        transition: color 0.15s ease, background 0.15s ease, border-left-color 0.15s ease;
        width: 100%;
        text-align: left;
    }

    .nav-item:hover {
        background: rgba(255 255 255 / 0.04);
        color: var(--echo-text-2);
        transform: none; /* override global button active scale */
    }

    .nav-item:active {
        transform: none;
    }

    .nav-item.active {
        border-left-color: var(--echo-silver);
        color: var(--echo-text-1);
        background: rgba(255 255 255 / 0.04);
    }

    .sidebar-footer {
        border-top: 1px solid var(--echo-border);
        padding-top: 0.75rem;
        margin-top: 0.75rem;
    }
</style>
