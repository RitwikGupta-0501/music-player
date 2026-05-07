<script lang="ts">
    let { activeView = $bindable("albums") } = $props<{ activeView?: string }>();
</script>

<div class="sidebar">
    <div>
        <h1 style="margin: 0;">Echo <span class="text-cyan">Engine</span></h1>
    </div>

    <nav style="display: flex; flex-direction: column; gap: 0.5rem; margin-top: 2rem;">
        <button 
            class={activeView === "albums" ? "primary" : "ghost"} 
            style="text-align: left;"
            onclick={() => activeView = "albums"}
        >
            💿 Local Albums
        </button>
        <button 
            class={activeView === "playlists" ? "primary" : "ghost"} 
            style="text-align: left;"
            onclick={() => activeView = "playlists"}
        >
            📝 Playlists
        </button>
        <button 
            class={activeView === "providers" ? "primary" : "ghost"} 
            style="text-align: left;"
            onclick={() => activeView = "providers"}
        >
            🌐 Network Providers
        </button>
        <button 
            class={activeView === "settings" ? "primary" : "ghost"} 
            style="text-align: left;"
            onclick={() => activeView = "settings"}
        >
            ⚙ Settings
        </button>
    </nav>
</div>
