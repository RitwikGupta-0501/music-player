<script lang="ts">
    import { Disc, ListMusic, Globe, Settings } from "lucide-svelte";
    let { activeView = $bindable("albums") } = $props<{ activeView?: string }>();
</script>

<div class="sidebar">
    <div>
        <h1 style="margin: 0; font-family: var(--font-display); font-weight: 700; font-size: 1.5rem; letter-spacing: -0.02em;">
            Echo <span class="text-cyan">Engine</span>
        </h1>
    </div>

    <nav class="nav-links">
        <button 
            class={activeView === "albums" ? "primary" : "ghost"} 
            onclick={() => activeView = "albums"}
        >
            <Disc size={18} />
            <span>Local Albums</span>
        </button>
        <button 
            class={activeView === "playlists" ? "primary" : "ghost"} 
            onclick={() => activeView = "playlists"}
        >
            <ListMusic size={18} />
            <span>Playlists</span>
        </button>
        <button 
            class={activeView === "providers" ? "primary" : "ghost"} 
            onclick={() => activeView = "providers"}
        >
            <Globe size={18} />
            <span>Network Providers</span>
        </button>
        <button 
            class={activeView === "settings" ? "primary" : "ghost"} 
            onclick={() => activeView = "settings"}
        >
            <Settings size={18} />
            <span>Settings</span>
        </button>
    </nav>
</div>

<style>
    .nav-links {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        margin-top: 2rem;
    }

    .nav-links button {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        text-align: left;
        width: 100%;
        padding: 0.75rem 1rem;
        border-radius: 8px;
        font-size: 0.95rem;
        font-family: var(--font-body);
        font-weight: 500;
        transition: all 0.2s ease;
    }

    .nav-links button.ghost {
        background: transparent;
        color: var(--color-chalk-muted);
        border: 1px solid transparent;
    }

    .nav-links button.ghost:hover {
        background: rgba(255, 255, 255, 0.05);
        color: #fff;
        border-color: rgba(197, 198, 199, 0.05);
    }

    .nav-links button.primary {
        background: rgba(102, 252, 241, 0.1);
        color: var(--color-cyan);
        border: 1px solid rgba(102, 252, 241, 0.2);
    }

    .nav-links button.primary:hover {
        background: rgba(102, 252, 241, 0.15);
        color: var(--color-cyan);
        box-shadow: 0 0 15px rgba(102, 252, 241, 0.15);
    }
</style>
