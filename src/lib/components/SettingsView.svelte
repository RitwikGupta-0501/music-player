<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { libraryStore } from "$lib/stores/library.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { CaretDown, Check } from "phosphor-svelte";
    import { onMount } from 'svelte';

    let activeTab = $state<"playback" | "appearance" | "data">("playback");
    
    let keepPlayingOnClear = $state(false);
    let trackClickBehavior = $state<"interrupt" | "clear" | "append">("interrupt");
    let loaded = $state(false);
    
    let isDropdownOpen = $state(false);

    const behaviorOptions = [
        { value: "interrupt", label: "Play Next & Switch" },
        { value: "clear", label: "Clear Queue & Play" },
        { value: "append", label: "Add to End of Queue" }
    ];

    onMount(async () => {
        try {
            const val = await invoke<string | null>("get_setting", { key: "keep_playing_on_queue_clear" });
            keepPlayingOnClear = val === "true";

            const clickVal = await invoke<string | null>("get_setting", { key: "track_click_behavior" });
            if (clickVal) trackClickBehavior = clickVal as "interrupt" | "clear" | "append";
        } catch (e) {
            console.error("Failed to load setting:", e);
        } finally {
            loaded = true;
        }
    });

    async function toggleKeepPlaying() {
        if (!loaded) return;
        keepPlayingOnClear = !keepPlayingOnClear;
        try {
            await invoke("set_setting", { key: "keep_playing_on_queue_clear", value: keepPlayingOnClear ? "true" : "false" });
        } catch (e) {
            console.error("Failed to save setting:", e);
            keepPlayingOnClear = !keepPlayingOnClear; // Revert
        }
    }

    async function selectBehavior(val: "interrupt" | "clear" | "append") {
        if (!loaded) return;
        trackClickBehavior = val;
        isDropdownOpen = false;
        
        import('$lib/stores/audio.svelte').then(({ audioStore }) => {
            audioStore.setTrackClickBehavior(val);
        });
    }

    function handleOutsideClick(e: MouseEvent) {
        if (isDropdownOpen) {
            isDropdownOpen = false;
        }
    }

    async function factoryReset() {
        const yes = confirm("Are you sure you want to completely wipe your library and settings?");
        if (yes) {
            try {
                await invoke("factory_reset");
                libraryStore.albums = [];
                libraryStore.playlists = [];
            } catch (e) {
                console.error("Factory reset failed:", e);
                alert("Failed to reset library.");
            }
        }
    }
</script>

<svelte:window onclick={handleOutsideClick} />

<div class="settings-layout">
    <!-- Left Pane: Navigation -->
    <aside class="settings-nav">
        <h2 class="settings-title">Settings</h2>
        
        <nav class="nav-list">
            <button 
                class="nav-item" 
                class:active={activeTab === "playback"} 
                onclick={() => activeTab = "playback"}
            >
                Playback
            </button>
            <button 
                class="nav-item" 
                class:active={activeTab === "appearance"} 
                onclick={() => activeTab = "appearance"}
            >
                Appearance
            </button>
            <button 
                class="nav-item" 
                class:active={activeTab === "data"} 
                onclick={() => activeTab = "data"}
            >
                Data & Privacy
            </button>
        </nav>
    </aside>

    <!-- Right Pane: Content -->
    <div class="settings-content" onclick={(e) => e.stopPropagation()}>
        <div class="content-container">
            {#if activeTab === "playback"}
                <section class="settings-section">
                    <h3 class="section-title">Audio & Playback</h3>
                    
                    <div class="setting-row">
                        <div class="setting-info">
                            <p class="setting-label">Keep Playing on Queue Clear</p>
                            <p class="setting-desc">Allow the current song to finish even if the upcoming queue is wiped.</p>
                        </div>
                        <label class="switch">
                            <input type="checkbox" checked={keepPlayingOnClear} onchange={toggleKeepPlaying} disabled={!loaded} />
                            <span class="slider round"></span>
                        </label>
                    </div>

                    <div class="setting-row">
                        <div class="setting-info">
                            <p class="setting-label">When clicking a track</p>
                            <p class="setting-desc">Behavior when playing a single track while a queue is active.</p>
                        </div>
                        
                        <div class="custom-select-container">
                            <button 
                                class="select-trigger" 
                                onclick={(e) => { e.stopPropagation(); isDropdownOpen = !isDropdownOpen; }}
                                disabled={!loaded}
                            >
                                <span>{behaviorOptions.find(o => o.value === trackClickBehavior)?.label || "Select..."}</span>
                                <CaretDown size={14} weight="bold" class={isDropdownOpen ? 'rotated' : ''} />
                            </button>

                            {#if isDropdownOpen}
                                <div class="custom-select-menu glass">
                                    {#each behaviorOptions as opt}
                                        <button 
                                            class="select-option" 
                                            class:selected={trackClickBehavior === opt.value}
                                            onclick={(e) => { e.stopPropagation(); selectBehavior(opt.value as any); }}
                                        >
                                            <span class="opt-label">{opt.label}</span>
                                            {#if trackClickBehavior === opt.value}
                                                <Check size={14} weight="bold" class="check-icon" />
                                            {/if}
                                        </button>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                    </div>
                </section>
            {:else if activeTab === "appearance"}
                <section class="settings-section">
                    <h3 class="section-title">Appearance</h3>
                    
                    <div class="setting-row">
                        <div class="setting-info">
                            <p class="setting-label">Glassy Player Bar</p>
                            <p class="setting-desc">Enable a sleek, semi-transparent frosted glass effect for the bottom player.</p>
                        </div>
                        <label class="switch">
                            <input type="checkbox" checked={settingsStore.glassyPlayerBar} onchange={(e) => settingsStore.setGlassyPlayerBar(e.currentTarget.checked)} disabled={!loaded} />
                            <span class="slider round"></span>
                        </label>
                    </div>
                </section>
            {:else if activeTab === "data"}
                <section class="settings-section">
                    <h3 class="section-title">Data Management</h3>
                    
                    <div class="setting-row" style="flex-direction: column; align-items: flex-start; gap: 1.5rem;">
                        <div class="setting-info">
                            <p class="setting-desc" style="font-size: 0.95rem;">
                                This will delete the local SQLite database and clear all cached artwork and extensions. Your actual music files will not be touched.
                            </p>
                        </div>
                        <button class="danger-btn" onclick={factoryReset}>
                            Factory Reset (Wipe Database)
                        </button>
                    </div>
                </section>
            {/if}
        </div>
    </div>
</div>

<style>
    .settings-layout {
        display: flex;
        height: 100vh;
        width: 100%;
    }

    /* Left Pane */
    .settings-nav {
        width: 250px;
        min-width: 250px;
        border-right: 1px solid var(--echo-border);
        padding: 3rem 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 2rem;
    }

    .settings-title {
        font-family: var(--echo-font-heading);
        font-size: 1.5rem;
        font-weight: 600;
        color: var(--echo-text-1);
        margin: 0;
        padding-left: 0.5rem;
    }

    .nav-list {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }

    .nav-item {
        background: transparent;
        border: none;
        color: var(--echo-text-2);
        font-family: var(--echo-font-body);
        font-size: 0.95rem;
        font-weight: 500;
        text-align: left;
        padding: 0.6rem 0.8rem;
        border-radius: 6px;
        cursor: pointer;
        transition: all 0.2s ease;
        position: relative;
    }

    .nav-item:hover:not(.active) {
        background: rgba(255, 255, 255, 0.05);
        color: var(--echo-text-1);
    }

    .nav-item.active {
        color: var(--echo-text-1);
        background: rgba(226, 169, 115, 0.08); /* subtle primary tint */
    }

    .nav-item.active::before {
        content: "";
        position: absolute;
        left: 0;
        top: 20%;
        bottom: 20%;
        width: 3px;
        background: var(--echo-primary);
        border-radius: 0 4px 4px 0;
    }

    /* Right Pane */
    .settings-content {
        flex: 1;
        overflow-y: auto;
        padding: 3rem 4rem;
        display: flex;
        flex-direction: column;
    }

    .content-container {
        max-width: 650px;
        width: 100%;
    }

    .settings-section {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        animation: fadeIn 0.2s ease;
    }

    .section-title {
        font-family: var(--echo-font-heading);
        font-size: 1.25rem;
        font-weight: 500;
        color: var(--echo-text-1);
        padding-bottom: 0.5rem;
        border-bottom: 1px solid var(--echo-border);
        margin-bottom: 0.5rem;
    }

    .setting-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem 0;
    }

    .setting-info {
        display: flex;
        flex-direction: column;
        gap: 0.3rem;
        margin-right: 2rem;
    }

    .setting-label {
        font-size: 1rem;
        font-weight: 500;
        color: var(--echo-text-1);
        margin: 0;
    }

    .setting-desc {
        font-size: 0.85rem;
        color: var(--echo-text-2);
        margin: 0;
        line-height: 1.5;
    }

    .danger-btn {
        background-color: rgba(220, 38, 38, 0.1);
        color: #ef4444;
        border: 1px solid rgba(220, 38, 38, 0.3);
        padding: 0.6rem 1.5rem;
        border-radius: 8px;
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s ease;
    }

    .danger-btn:hover {
        background-color: rgba(220, 38, 38, 0.2);
        border-color: rgba(220, 38, 38, 0.5);
    }

    /* Dropdown UI */
    .custom-select-container {
        position: relative;
        min-width: 200px;
    }

    .select-trigger {
        width: 100%;
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: var(--echo-surface);
        border: 1px solid var(--echo-border-medium);
        color: var(--echo-text-1);
        padding: 0.6rem 1rem;
        border-radius: 8px;
        font-family: var(--echo-font-body);
        font-size: 0.9rem;
        cursor: pointer;
        transition: all 0.2s ease;
    }

    .select-trigger:hover:not(:disabled) {
        border-color: var(--echo-text-2);
        background: rgba(255, 255, 255, 0.04);
    }

    .select-trigger :global(svg) {
        transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1);
        color: var(--echo-text-2);
    }

    .select-trigger :global(svg.rotated) {
        transform: rotate(180deg);
    }

    .custom-select-menu {
        position: absolute;
        top: calc(100% + 0.5rem);
        right: 0;
        width: max-content;
        min-width: 100%;
        display: flex;
        flex-direction: column;
        background: var(--echo-surface);
        border: 1px solid var(--echo-border-medium);
        border-radius: 8px;
        padding: 0.3rem;
        z-index: 50;
        box-shadow: 0 10px 30px -10px rgba(0, 0, 0, 0.6);
        animation: slideDown 0.15s cubic-bezier(0.16, 1, 0.3, 1) forwards;
        transform-origin: top center;
    }

    .select-option {
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;
        background: transparent;
        border: none;
        color: var(--echo-text-2);
        padding: 0.6rem 0.8rem;
        border-radius: 6px;
        font-family: var(--echo-font-body);
        font-size: 0.85rem;
        cursor: pointer;
        transition: all 0.15s ease;
        text-align: left;
    }

    .select-option:hover {
        background: rgba(255, 255, 255, 0.05);
        color: var(--echo-text-1);
    }

    .select-option.selected {
        color: var(--echo-primary);
        background: rgba(255, 255, 255, 0.02);
        font-weight: 500;
    }

    .select-option :global(.check-icon) {
        color: var(--echo-primary);
        margin-left: 1rem;
    }

    /* Switch CSS */
    .switch {
        position: relative;
        display: inline-block;
        width: 44px;
        height: 24px;
        flex-shrink: 0;
    }
    .switch input {
        opacity: 0;
        width: 0;
        height: 0;
    }
    .slider {
        position: absolute;
        cursor: pointer;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: var(--echo-raised);
        border: 1px solid var(--echo-border-medium);
        transition: .4s;
    }
    .slider:before {
        position: absolute;
        content: "";
        height: 16px;
        width: 16px;
        left: 3px;
        bottom: 3px;
        background-color: var(--echo-text-2);
        transition: .4s;
    }
    input:checked + .slider {
        background-color: var(--echo-primary);
        border-color: var(--echo-primary);
    }
    input:checked + .slider:before {
        transform: translateX(20px);
        background-color: var(--echo-void);
    }
    .slider.round {
        border-radius: 34px;
    }
    .slider.round:before {
        border-radius: 50%;
    }

    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(5px); }
        to { opacity: 1; transform: translateY(0); }
    }
    @keyframes slideDown {
        0% { opacity: 0; transform: translateY(-4px) scale(0.98); }
        100% { opacity: 1; transform: translateY(0) scale(1); }
    }
</style>
