<script lang="ts">
    import { List, House, Disc, PuzzlePiece, Gear } from "phosphor-svelte";
    
    let { activeView = $bindable("albums") } = $props<{ activeView?: string }>();
    
    let sidebarOpen = $state(false);

    function toggleSidebar() {
        sidebarOpen = !sidebarOpen;
    }
</script>

<aside class="sidebar" class:open={sidebarOpen}>
    <div class="sidebar-content">
        <!-- Menu Toggle & Brand -->
        <div class="sidebar-header">
            <button class="toggle-btn" onclick={toggleSidebar}>
                <List size={20} weight="bold" />
            </button>
            <div class="brand">
                <span class="wordmark">Sonic Topography</span>
            </div>
        </div>

        <!-- Primary navigation -->
        <nav class="sidebar-nav">
            <button
                class="nav-item"
                class:active={activeView === "home"}
                onclick={() => activeView = "home"}
            >
                <div class="icon-container">
                    <House size={24} weight={activeView === "home" ? "fill" : "regular"} />
                </div>
                <span class="label">Home</span>
            </button>
            <button
                class="nav-item"
                class:active={activeView === "albums" || activeView === "playlists"}
                onclick={() => activeView = "albums"}
            >
                <div class="icon-container">
                    <Disc size={24} weight={activeView === "albums" || activeView === "playlists" ? "fill" : "regular"} />
                </div>
                <span class="label">Library</span>
            </button>
            <button
                class="nav-item"
                class:active={activeView === "providers"}
                onclick={() => activeView = "providers"}
            >
                <div class="icon-container">
                    <PuzzlePiece size={24} weight={activeView === "providers" ? "fill" : "regular"} />
                </div>
                <span class="label">Extensions</span>
            </button>
        </nav>
    </div>

    <!-- Settings pinned to bottom -->
    <div class="sidebar-footer">
        <button
            class="nav-item"
            class:active={activeView === "settings"}
            onclick={() => activeView = "settings"}
        >
            <div class="icon-container">
                <Gear size={24} weight={activeView === "settings" ? "fill" : "regular"} />
            </div>
            <span class="label">Settings</span>
        </button>
    </div>
</aside>

<style>
    .sidebar {
        background: rgba(0, 0, 0, 0.4);
        backdrop-filter: blur(40px);
        -webkit-backdrop-filter: blur(40px);
        border-right: 1px solid rgba(255, 255, 255, 0.05);
        height: 100vh;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        padding: 2rem 0;
        position: sticky;
        top: 0;
        user-select: none;
        width: 80px;
        transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        z-index: 30;
        flex-shrink: 0;
        overflow: hidden;
    }

    .sidebar.open {
        width: 256px; /* w-64 */
    }

    /* Reduced-transparency fallback */
    @media (prefers-reduced-transparency: reduce) {
        .sidebar {
            background: var(--echo-surface);
            backdrop-filter: none;
            -webkit-backdrop-filter: none;
        }
    }

    .sidebar-content {
        display: flex;
        flex-direction: column;
        gap: 2rem;
        width: 100%;
        padding: 0 1rem;
    }

    .sidebar-header {
        display: flex;
        align-items: center;
        gap: 1rem;
        width: 100%;
    }

    .toggle-btn {
        width: 48px;
        height: 48px;
        flex-shrink: 0;
        border-radius: 9999px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(255, 255, 255, 0.05);
        color: var(--echo-text-2);
        cursor: pointer;
        transition: background-color 0.2s, color 0.2s;
        padding: 0;
    }

    .toggle-btn:hover {
        background: rgba(255, 255, 255, 0.1);
        color: var(--echo-text-1);
    }

    .brand {
        display: flex;
        align-items: center;
    }

    .wordmark {
        font-family: var(--echo-font-heading);
        font-size: 1.25rem; /* text-xl */
        font-weight: 400;
        font-style: italic;
        color: var(--echo-text-1);
        white-space: nowrap;
        letter-spacing: 0.025em;
    }

    .sidebar-nav {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        width: 100%;
    }

    .nav-item {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 0.25rem; /* p-1 */
        background: transparent;
        border: 1px solid transparent;
        border-radius: 9999px; /* Pill shape */
        color: var(--echo-text-2);
        cursor: pointer;
        transition: all 0.2s ease;
        width: 100%;
        text-align: left;
        overflow: hidden;
    }

    .icon-container {
        width: 40px;
        height: 40px;
        flex-shrink: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 9999px;
    }
    
    .label {
        white-space: nowrap;
        font-size: 0.9375rem; /* 15px */
        font-weight: 500;
        font-family: var(--echo-font-body);
        letter-spacing: 0.025em;
    }

    .nav-item:hover:not(.active) {
        background: rgba(255, 255, 255, 0.05);
        color: var(--echo-text-1);
    }

    .nav-item.active {
        color: var(--echo-primary);
        background: rgba(226, 169, 115, 0.1);
        border: 1px solid rgba(226, 169, 115, 0.2);
        box-shadow: 0 0 15px rgba(226, 169, 115, 0.1);
    }

    .sidebar-footer {
        padding: 0 1rem;
        width: 100%;
    }
</style>
