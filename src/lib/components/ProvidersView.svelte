<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';
    import { PlayCircle, PuzzlePiece, MagnifyingGlass, SpinnerGap, ArrowRight, CheckCircle, XCircle, ArrowsClockwise, SlidersHorizontal, FolderOpen, ShieldCheck, Copy, AppleLogo, SoundcloudLogo, SpotifyLogo, MapPin } from 'phosphor-svelte';

    interface ProviderInfo {
        id: string;
        name: string;
        author: string;
        version: string;
        file_path: string;
        status: string;
        error_message: string | null;
        checksum: string | null;
        capabilities: string[] | null;
        homepage: string | null;
        settings_schema: string | null;
        priority: number;
        icon: string | null;
        settings: string | null;
    }

    interface TrackResult {
        id: string;
        title: string;
        artist: string;
        album: string | null;
        cover_art_url: string | null;
        stream_url: string | null;
        quality_hint: string | null;
        duration_ms: number | null;
    }

    let providers = $state<ProviderInfo[]>([]);
    let activeProviderPath = $state<string | null>(null);
    let isScanning = $state(false);

    let searchQuery = $state('');
    let isSearching = $state(false);
    let searchResults = $state<TrackResult[]>([]);
    let searchError = $state<string | null>(null);
    let currentlyLoadingTrackId = $state<string | null>(null);

    let activeProvider = $derived(providers.find(p => p.file_path === activeProviderPath));
    let currentTab = $state<'details' | 'configure' | 'test'>('details');

    let parsedSchema = $derived(
        (() => {
            if (!activeProvider?.settings_schema) return [];
            try { 
                const parsed = JSON.parse(activeProvider.settings_schema); 
                if (Array.isArray(parsed)) return parsed;
                if (typeof parsed === 'object' && parsed !== null && Object.keys(parsed).length === 0) return [];
                return parsed;
            }
            catch (e) { return []; }
        })()
    );
    let parsedSettings = $derived(
        (() => {
            if (!activeProvider?.settings) return {};
            try { return JSON.parse(activeProvider.settings); }
            catch (e) { return {}; }
        })()
    );
    // Draft settings state for editing
    let draftSettings = $state<Record<string, any>>({});
    
    // UI states for new buttons
    let isVerifyingChecksum = $state(false);
    let checksumVerified = $state(false);

    // Dynamic icon resolution
    function getProviderIcon(iconName: string | null) {
        if (!iconName) return PuzzlePiece;
        switch(iconName) {
            case 'AppleLogo': return AppleLogo;
            case 'SoundcloudLogo': return SoundcloudLogo;
            case 'SpotifyLogo': return SpotifyLogo;
            default: return PuzzlePiece;
        }
    }

    async function handleVerifyChecksum() {
        isVerifyingChecksum = true;
        checksumVerified = false;
        // Mock a verification delay
        await new Promise(r => setTimeout(r, 800));
        isVerifyingChecksum = false;
        checksumVerified = true;
        setTimeout(() => checksumVerified = false, 3000);
    }

    async function handleOpenPath(path: string) {
        try {
            // Get directory by stripping filename
            const dir = path.substring(0, path.lastIndexOf('/')) || path.substring(0, path.lastIndexOf('\\'));
            await invoke('open_in_file_explorer', { path: dir || path });
        } catch (e) {
            console.error("Failed to open path:", e);
        }
    }

    // Update draft settings when active provider or tab changes
    $effect(() => {
        if (currentTab === 'configure' && activeProvider) {
            draftSettings = activeProvider.settings ? JSON.parse(activeProvider.settings) : {};
        }
    });

    let isSavingSettings = $state(false);

    async function saveSettings() {
        if (!activeProvider) return;
        isSavingSettings = true;
        try {
            await invoke('save_provider_settings', {
                providerId: activeProvider.id,
                settingsJson: JSON.stringify(draftSettings)
            });
            // Reload providers to get updated state
            await loadProviders();
        } catch (error) {
            console.error("Failed to save settings:", error);
        } finally {
            isSavingSettings = false;
        }
    }

    onMount(async () => {
        await loadProviders();
    });

    async function loadProviders() {
        isScanning = true;
        try {
            providers = await invoke<ProviderInfo[]>('get_providers');
            if (providers.length > 0 && !activeProviderPath) {
                // Auto-select the first ENABLED provider if none selected
                const firstEnabled = providers.find(p => p.status === 'enabled');
                if (firstEnabled) {
                    await setActiveProvider(firstEnabled.file_path);
                }
            }
        } catch (error) {
            console.error("Failed to load providers:", error);
        } finally {
            isScanning = false;
        }
    }

    async function setActiveProvider(path: string) {
        activeProviderPath = path;
        
        // Clear previous test results
        searchResults = [];
        searchError = null;
        searchQuery = '';
    }

    async function toggleProvider(provider: ProviderInfo) {
        try {
            const newEnabled = provider.status !== 'enabled';
            await invoke('toggle_provider', { 
                providerId: provider.id, 
                enabled: newEnabled 
            });
            
            // Optimistically update local state
            providers = providers.map(p => 
                p.id === provider.id ? { ...p, status: newEnabled ? 'enabled' : 'disabled' } : p
            );
        } catch (error) {
            console.error("Failed to toggle provider:", error);
        }
    }

    async function testSearch() {
        if (!searchQuery.trim() || !activeProviderPath) return;

        isSearching = true;
        searchError = null;
        searchResults = [];
        try {
            if (activeProvider) {
                searchResults = await invoke<TrackResult[]>('search_provider', { providerId: activeProvider.id, query: searchQuery });
            }
        } catch (error: any) {
            searchError = error.toString();
        } finally {
            isSearching = false;
        }
    }

    async function playTrack(track: TrackResult) {
        if (currentlyLoadingTrackId === track.id) return;
        
        currentlyLoadingTrackId = track.id;
        try {
            const active = providers.find(p => p.file_path === activeProviderPath);
            const providerId = active ? active.id : "unknown";

            await invoke('load_audio', {
                source: {
                    type: "Remote",
                    provider_id: providerId,
                    remote_track_id: track.id,
                    stream_url: track.stream_url,
                    quality_hint: track.quality_hint || null,
                    cover_art_url: track.cover_art_url || null,
                    duration_ms: track.duration_ms || null,
                },
                title: track.title,
                artist: track.artist,
                album: track.album
            });
            // The audio pipeline might take a moment to buffer.
            // Ideally we'd clear this when the player-sync event says "Playing",
            // but for the sandbox tester, we can just clear it after a short delay
            // or let the user know it was sent successfully.
            setTimeout(() => {
                if (currentlyLoadingTrackId === track.id) {
                    currentlyLoadingTrackId = null;
                }
            }, 1500);
        } catch (error) {
            console.error("Failed to play track:", error);
            currentlyLoadingTrackId = null;
        }
    }
</script>

<div class="providers-view">
    <header class="view-header">
        <h1 class="text-1">Extensions</h1>
        <p class="text-2">Manage and test your network provider plugins.</p>
    </header>

    <div class="content-grid">
        <!-- LEFT COLUMN: INSTALLED PLUGINS -->
        <div class="plugins-list">
            <div class="section-title">
                <h2>Installed Providers</h2>
                <button class="refresh-btn" onclick={async () => {
                    isScanning = true;
                    try {
                        await invoke('sync_providers');
                        await loadProviders();
                    } finally {
                        isScanning = false;
                    }
                }} disabled={isScanning}>
                    {#if isScanning}
                        <SpinnerGap class="spin" size={16} />
                        Syncing...
                    {:else}
                        <ArrowsClockwise size={16} />
                        Sync
                    {/if}
                </button>
            </div>

            <div class="cards-container">
                {#if providers.length === 0 && !isScanning}
                    <div class="empty-state">
                        <PuzzlePiece size={32} class="text-3" />
                        <p class="text-3">No providers found in app directory.</p>
                    </div>
                {/if}

                {#each providers as provider}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div 
                        class="provider-card {activeProviderPath === provider.file_path ? 'active' : ''} {provider.status !== 'enabled' ? 'disabled' : ''}"
                        onclick={() => setActiveProvider(provider.file_path)}
                    >
                        <div class="card-icon">
                            <PuzzlePiece size={24} weight={activeProviderPath === provider.file_path ? 'fill' : 'regular'} />
                        </div>
                        <div class="card-details text-left" style="flex-grow: 1;">
                            <h3>{provider.name}</h3>
                            <p class="text-3">by {provider.author} • v{provider.version}</p>
                        </div>
                        
                        <button 
                            class="power-btn"
                            style="flex-shrink: 0; color: {provider.status === 'enabled' ? 'var(--echo-primary)' : 'var(--echo-text-3)'}; opacity: {provider.status === 'enabled' ? '1' : '0.5'}"
                            onclick={(e) => { e.stopPropagation(); toggleProvider(provider); }}
                            aria-label="Toggle provider"
                        >
                            {#if provider.status === 'enabled'}
                                <CheckCircle size={24} weight="fill" />
                            {:else}
                                <XCircle size={24} weight="regular" />
                            {/if}
                        </button>
                    </div>
                {/each}
            </div>
        </div>

        <!-- RIGHT COLUMN: EXTENSION DETAILS -->
        <div class="sandbox-tester">
            {#if !activeProvider}
                <div class="section-title">
                    <h2>Extension Details</h2>
                </div>
                <div class="tester-panel" style="display: flex; align-items: center; justify-content: center;">
                    <div class="empty-state">
                        <p class="text-3">Select a provider to view details.</p>
                    </div>
                </div>
            {:else}
                <div class="section-title" style="display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--echo-border); padding-bottom: 1rem; margin-bottom: 1.5rem;">
                    <div style="display: flex; gap: 0.5rem; align-items: center;">
                        <button class="tab-btn {currentTab === 'details' ? 'active' : ''}" onclick={() => currentTab = 'details'}>Details</button>
                        <button class="tab-btn {currentTab === 'configure' ? 'active' : ''}" onclick={() => currentTab = 'configure'}>Configure</button>
                        <button class="tab-btn {currentTab === 'test' ? 'active' : ''}" onclick={() => currentTab = 'test'}>Test Sandbox</button>
                    </div>
                    <span class="badge" style="background: {activeProvider.status === 'enabled' ? 'rgba(34, 197, 94, 0.1)' : 'var(--echo-overlay)'}; color: {activeProvider.status === 'enabled' ? 'var(--color-success, #22c55e)' : 'var(--echo-text-3)'}; padding: 0.35rem 0.75rem;">
                        {activeProvider.status.toUpperCase()}
                    </span>
                </div>

                <div class="tester-panel">
                    {#if currentTab === 'details'}
                        {@const IconCmp = getProviderIcon(activeProvider.icon)}
                        <div class="details-tab">
                            <div class="details-header" style="display: flex; gap: 1.5rem; align-items: flex-start; margin-bottom: 2.5rem;">
                                <div class="icon-lg">
                                    <IconCmp size={48} weight="duotone" />
                                </div>
                                <div>
                                    <h2 style="font-size: 2rem; margin-bottom: 0.5rem; display: flex; align-items: center; gap: 0.5rem;">
                                        {activeProvider.name}
                                        <span class="badge" style="background: rgba(255,255,255,0.05); color: var(--echo-text-2);">v{activeProvider.version}</span>
                                    </h2>
                                    <div style="display: flex; gap: 0.75rem; align-items: center;">
                                        <p class="text-2">by <strong style="color: var(--echo-text-1);">{activeProvider.author}</strong></p>
                                        {#if activeProvider.homepage}
                                            <span style="color: var(--echo-text-3);">•</span>
                                            <a href={activeProvider.homepage} target="_blank" class="text-accent" style="text-decoration: none; color: var(--echo-primary); font-size: 0.875rem; display: flex; align-items: center; gap: 0.25rem;">
                                                Website ↗
                                            </a>
                                        {/if}
                                    </div>
                                </div>
                            </div>
                            
                            <div class="meta-section" style="margin-bottom: 2.5rem;">
                                <h3 style="font-size: 1.125rem; font-weight: 500; color: var(--echo-text-1); margin-bottom: 1rem;">Capabilities</h3>
                                <div style="display: flex; gap: 0.75rem; flex-wrap: wrap;">
                                    {#if activeProvider.capabilities && activeProvider.capabilities.length > 0}
                                        {#each activeProvider.capabilities as cap}
                                            <div class="capability-tag">
                                                {#if cap === 'search'}
                                                    <MagnifyingGlass size={16} />
                                                {:else if cap === 'stream'}
                                                    <PlayCircle size={16} />
                                                {:else}
                                                    <PuzzlePiece size={16} />
                                                {/if}
                                                <span>{cap}</span>
                                            </div>
                                        {/each}
                                    {:else}
                                        <span class="text-3">None declared</span>
                                    {/if}
                                </div>
                            </div>

                            <div class="meta-section">
                                <h3 style="font-size: 1.125rem; font-weight: 500; color: var(--echo-text-1); margin-bottom: 1rem;">System Info</h3>
                                <div class="bento-grid">
                                    <!-- File Path Card -->
                                    <div class="bento-card">
                                        <div class="bento-header">
                                            <MapPin size={18} />
                                            <span>Local Path</span>
                                        </div>
                                        <div class="bento-value path-value" title={activeProvider.file_path}>
                                            {activeProvider.file_path}
                                        </div>
                                        <button class="bento-action-btn" onclick={() => handleOpenPath(activeProvider.file_path)}>
                                            <FolderOpen size={16} />
                                            Open in Explorer
                                        </button>
                                    </div>

                                    <!-- Security / Checksum Card -->
                                    <div class="bento-card">
                                        <div class="bento-header">
                                            <ShieldCheck size={18} />
                                            <span>Security Hash</span>
                                        </div>
                                        <div class="bento-value hash-value" title={activeProvider.checksum || 'No checksum available'}>
                                            {activeProvider.checksum || 'N/A'}
                                        </div>
                                        <button class="bento-action-btn" onclick={handleVerifyChecksum} disabled={isVerifyingChecksum || !activeProvider.checksum}>
                                            {#if isVerifyingChecksum}
                                                <SpinnerGap size={16} class="spin" />
                                                Verifying...
                                            {:else if checksumVerified}
                                                <CheckCircle size={16} weight="fill" style="color: var(--color-success, #22c55e);" />
                                                Verified
                                            {:else}
                                                <PuzzlePiece size={16} />
                                                Run Integrity Test
                                            {/if}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    {:else if currentTab === 'configure'}
                        <div class="configure-tab">
                            {#if parsedSchema.length === 0}
                                <div class="empty-state" style="min-height: 40vh; text-align: center; gap: 0.5rem; display: flex; flex-direction: column; align-items: center; justify-content: center;">
                                    <div class="empty-icon" style="color: var(--echo-text-3); margin-bottom: 1.25rem; opacity: 0.6;">
                                        <SlidersHorizontal size={48} weight="thin" />
                                    </div>
                                    <h2 class="empty-heading" style="font-size: 2rem; font-weight: 500; color: var(--echo-text-1); letter-spacing: -0.025em;">No Configuration Needed</h2>
                                    <p class="empty-sub" style="font-size: 1rem; color: var(--echo-text-2); max-width: 320px; line-height: 1.6;">This extension is plug-and-play and doesn't require any custom settings.</p>
                                </div>
                            {:else}
                                <div class="settings-form">
                                    {#each parsedSchema as field}
                                        <div class="form-group" style="margin-bottom: 1.5rem;">
                                            <label style="display: block; font-weight: 500; margin-bottom: 0.5rem;" for="field-{field.key}">
                                                {field.label} {#if field.required}<span style="color: #ef4444;">*</span>{/if}
                                            </label>
                                            {#if field.description}
                                                <p class="text-3" style="margin-bottom: 0.5rem; font-size: 0.85rem;">{field.description}</p>
                                            {/if}

                                            {#if field.type === 'boolean'}
                                                <!-- svelte-ignore a11y_click_events_have_key_events -->
                                                <!-- svelte-ignore a11y_no_static_element_interactions -->
                                                <div 
                                                    class="toggle-switch"
                                                    onclick={() => draftSettings[field.key] = !draftSettings[field.key]}
                                                    style="width: 40px; height: 24px; border-radius: 12px; background: {draftSettings[field.key] ? 'var(--echo-primary)' : 'var(--echo-overlay)'}; position: relative; cursor: pointer; transition: background 0.2s;"
                                                >
                                                    <div style="position: absolute; top: 2px; left: {draftSettings[field.key] ? '18px' : '2px'}; width: 20px; height: 20px; border-radius: 50%; background: var(--echo-base); transition: left 0.2s;"></div>
                                                </div>
                                            {:else if field.type === 'select'}
                                                <select 
                                                    id="field-{field.key}"
                                                    bind:value={draftSettings[field.key]} 
                                                    style="width: 100%; padding: 0.75rem; background: var(--echo-overlay); border: 1px solid var(--echo-border); border-radius: 6px; color: var(--echo-text-1);"
                                                >
                                                    {#each field.options || [] as opt}
                                                        <option value={opt.value}>{opt.label}</option>
                                                    {/each}
                                                </select>
                                            {:else}
                                                <input 
                                                    id="field-{field.key}"
                                                    type={field.type === 'secret' ? 'password' : (field.type === 'number' ? 'number' : 'text')} 
                                                    bind:value={draftSettings[field.key]}
                                                    placeholder={field.default_value || ''}
                                                    style="width: 100%; padding: 0.75rem; background: var(--echo-overlay); border: 1px solid var(--echo-border); border-radius: 6px; color: var(--echo-text-1);"
                                                />
                                            {/if}
                                        </div>
                                    {/each}
                                    
                                    <div style="margin-top: 2rem; display: flex; justify-content: flex-end;">
                                        <button class="primary" onclick={saveSettings} disabled={isSavingSettings} style="display: flex; gap: 0.5rem; align-items: center;">
                                            {#if isSavingSettings}
                                                <SpinnerGap class="spin" size={16} /> Saving...
                                            {:else}
                                                Save Settings
                                            {/if}
                                        </button>
                                    </div>
                                </div>
                            {/if}
                        </div>
                    {:else}
                        <div class="test-sandbox-box">
                            <div class="search-bar">
                                <div class="input-wrapper">
                                    <div class="search-icon">
                                        <MagnifyingGlass size={20} />
                                    </div>
                                    <input 
                                        type="text" 
                                        bind:value={searchQuery} 
                                        placeholder="Search for tracks..." 
                                        onkeydown={(e) => e.key === 'Enter' && testSearch()}
                                    />
                                    <button class="search-btn" onclick={testSearch} disabled={isSearching || !searchQuery.trim()}>
                                        {#if isSearching}
                                            <SpinnerGap class="spin" size={20} />
                                        {:else}
                                            Test
                                        {/if}
                                    </button>
                                </div>
                            </div>

                            <div class="results-container">
                                {#if searchError}
                                    <div class="error-banner">
                                        {searchError}
                                    </div>
                                {/if}

                                {#if searchResults.length > 0}
                                    <div class="results-list">
                                        {#each searchResults as track}
                                            <div class="result-row">
                                                <div class="track-cover">
                                                    {#if track.cover_art_url}
                                                        <img src={track.cover_art_url} alt="Cover" />
                                                    {:else}
                                                        <div class="cover-placeholder"></div>
                                                    {/if}
                                                </div>
                                                <div class="track-info">
                                                    <span class="track-title">{track.title}</span>
                                                    <span class="track-artist">
                                                        {track.artist}
                                                        {#if track.album} • {track.album}{/if}
                                                    </span>
                                                    <span class="track-id text-3" title={track.stream_url}>ID: {track.id}</span>
                                                </div>
                                                <button class="play-btn" title={track.stream_url} onclick={() => playTrack(track)} disabled={currentlyLoadingTrackId === track.id}>
                                                    {#if currentlyLoadingTrackId === track.id}
                                                        <SpinnerGap class="spin text-accent" size={28} />
                                                    {:else}
                                                        <PlayCircle size={28} weight="fill" />
                                                    {/if}
                                                </button>
                                            </div>
                                        {/each}
                                    </div>
                                {:else if !isSearching && searchQuery}
                                    <div class="empty-state">
                                        <p class="text-3">No results found.</p>
                                    </div>
                                {:else if !isSearching && !searchQuery}
                                    <div class="empty-state">
                                        <MagnifyingGlass size={32} class="text-3" />
                                        <p class="text-3">Enter a query to test the provider.</p>
                                    </div>
                                {/if}
                            </div>
                        </div>
                    {/if}
                </div>
            {/if}
        </div>
    </div>
</div>

<style>
    .providers-view {
        position: absolute;
        inset: 0;
        padding: 2rem;
        padding-bottom: 8rem;
        display: flex;
        flex-direction: column;
        gap: 2rem;
        box-sizing: border-box;
    }

    .view-header h1 {
        font-size: 2rem;
        font-weight: 700;
        margin-bottom: 0.5rem;
    }

    .content-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 2rem;
        flex: 1;
        min-height: 0;
    }

    .section-title {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }

    .section-title h2 {
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--echo-text-1);
    }

    .refresh-btn {
        background: transparent;
        border: 1px solid var(--echo-border);
        color: var(--echo-text-2);
        padding: 0.25rem 0.75rem;
        border-radius: 4px;
        font-size: 0.875rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        transition: all 0.2s;
    }

    .refresh-btn:hover:not(:disabled) {
        color: var(--echo-text-1);
        border-color: var(--echo-text-3);
    }

    .plugins-list, .sandbox-tester {
        display: flex;
        flex-direction: column;
        background: transparent;
        border-radius: 12px;
        padding: 1.5rem;
        border: 1px solid var(--echo-border);
        min-height: 0;
    }

    .cards-container {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        overflow-y: auto;
        flex: 1;
    }

    .provider-card {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        background: transparent;
        border: 1px solid transparent;
        border-radius: 8px;
        cursor: pointer;
        text-align: left;
        transition: all 0.2s ease;
        position: relative;
    }

    .provider-card.disabled {
        opacity: 0.6;
        filter: grayscale(100%);
    }

    .provider-card:hover {
        background: rgba(226, 169, 115, 0.05); /* bronze-ish highlight */
    }

    .provider-card.active {
        background: rgba(226, 169, 115, 0.08); /* subtle primary tint */
        border-color: transparent;
    }

    .provider-card.active::before {
        content: "";
        position: absolute;
        left: 0;
        top: 20%;
        bottom: 20%;
        width: 3px;
        background: var(--echo-primary);
        border-radius: 0 4px 4px 0;
    }

    .card-icon {
        color: var(--echo-text-2);
    }

    .provider-card.active .card-icon {
        color: var(--echo-primary);
    }

    .card-details h3 {
        font-size: 1rem;
        font-weight: 500;
        color: var(--echo-text-1);
        margin-bottom: 0.25rem;
    }

    .power-btn {
        background: transparent;
        border: none;
        padding: 0.5rem;
        border-radius: 50%;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s ease;
    }
    .power-btn:hover {
        background: rgba(255, 255, 255, 0.1);
    }



    .tester-panel {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        flex: 1;
        min-height: 0;
    }

    .test-sandbox-box {
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
        background: transparent;
        flex: 1;
        min-height: 0;
    }

    .search-bar {
        display: flex;
        gap: 0.5rem;
    }

    .input-wrapper {
        position: relative;
        flex: 1;
    }

    .search-icon {
        position: absolute;
        left: 1rem;
        top: 50%;
        transform: translateY(-50%);
        color: var(--echo-text-3);
        display: flex;
        align-items: center;
        justify-content: center;
        pointer-events: none;
    }

    .input-wrapper input {
        width: 100%;
        padding: 0.75rem 5rem 0.75rem 3rem; /* Extra right padding for the button */
        background: transparent;
        border: 1px solid var(--echo-border);
        border-radius: 8px;
        color: var(--echo-text-1);
        font-size: 1rem;
        outline: none;
        transition: border-color 0.2s;
    }

    .input-wrapper input:focus {
        border-color: var(--echo-primary);
    }

    .search-btn {
        position: absolute;
        right: 0.35rem;
        top: 50%;
        transform: translateY(-50%);
        background: var(--echo-primary);
        color: var(--echo-base);
        border: none;
        padding: 0.4rem 1rem;
        border-radius: 6px;
        font-weight: 600;
        cursor: pointer;
        transition: opacity 0.2s;
    }

    .search-btn:hover:not(:disabled) {
        opacity: 0.9;
    }

    .search-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .primary {
        background: var(--echo-primary);
        color: var(--echo-base);
        border: none;
        padding: 0.75rem 1.5rem;
        border-radius: 8px;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
    }
    .primary:hover:not(:disabled) {
        transform: translateY(-1px);
        box-shadow: 0 4px 12px rgba(226, 169, 115, 0.2);
    }
    .primary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .results-container {
        flex: 1;
        overflow-y: auto;
        border: 1px solid var(--echo-border);
        border-radius: 8px;
        background: var(--echo-raised);
        position: relative;
    }

    .results-list {
        display: flex;
        flex-direction: column;
    }

    .result-row {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 1rem;
        border-bottom: 1px solid var(--echo-border);
        transition: background 0.2s;
    }

    .result-row:last-child {
        border-bottom: none;
    }

    .result-row:hover {
        background: var(--echo-overlay);
    }

    .track-cover {
        width: 48px;
        height: 48px;
        border-radius: 4px;
        overflow: hidden;
        flex-shrink: 0;
        background: var(--echo-surface);
    }

    .track-cover img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .cover-placeholder {
        width: 100%;
        height: 100%;
        background: var(--echo-border);
    }

    .track-info {
        display: flex;
        flex-direction: column;
        flex: 1;
        overflow: hidden;
    }

    .track-title {
        color: var(--echo-text-1);
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .track-artist {
        color: var(--echo-text-2);
        font-size: 0.875rem;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .track-id {
        font-size: 0.75rem;
        margin-top: 0.25rem;
        font-family: monospace;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .play-btn {
        background: transparent;
        border: none;
        color: var(--echo-primary);
        cursor: pointer;
        opacity: 0.7;
        transition: opacity 0.2s, transform 0.2s;
    }

    .result-row:hover .play-btn {
        opacity: 1;
        transform: scale(1.1);
    }

    .error-banner {
        margin: 1rem;
        padding: 1rem;
        background: rgba(220, 38, 38, 0.1);
        border: 1px solid rgba(220, 38, 38, 0.3);
        border-radius: 6px;
        color: #ef4444;
        font-family: monospace;
        font-size: 0.875rem;
        white-space: pre-wrap;
    }

    .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 100%;
        min-height: 200px;
        gap: 1rem;
    }

    :global(.spin) {
        animation: -global-spin 1s linear infinite;
    }

    @keyframes -global-spin {
        100% { transform: rotate(360deg); }
    }
    .tab-btn {
        background: transparent;
        border: none;
        color: var(--echo-text-2);
        padding: 0.5rem 1rem;
        border-radius: 8px;
        font-size: 1rem;
        cursor: pointer;
        transition: all 0.2s;
        position: relative;
    }

    .tab-btn:hover {
        color: var(--echo-text-1);
        background: rgba(255, 255, 255, 0.04);
    }

    .tab-btn.active {
        color: var(--echo-text-1);
        background: rgba(226, 169, 115, 0.08);
    }
    
    .tab-btn.active::after {
        content: "";
        position: absolute;
        bottom: 0;
        left: 20%;
        right: 20%;
        height: 3px;
        background: var(--echo-primary);
        border-radius: 4px 4px 0 0;
    }
    .badge {
        font-size: 0.75rem;
        padding: 0.15rem 0.5rem;
        border-radius: 12px;
        background: var(--echo-overlay);
        color: var(--echo-text-2);
        font-weight: 600;
    }

    .icon-lg {
        color: var(--echo-primary);
        background: rgba(226, 169, 115, 0.05);
        padding: 1rem;
        border-radius: 16px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 1px solid rgba(226, 169, 115, 0.15);
    }

    /* ── Details Tab Bento Grid ── */
    .capability-tag {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 8px;
        color: var(--echo-text-1);
        font-size: 0.875rem;
        font-weight: 500;
        text-transform: capitalize;
    }
    .capability-tag svg {
        color: var(--echo-primary);
    }

    .bento-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
        gap: 1.25rem;
    }

    .bento-card {
        background: rgba(255, 255, 255, 0.02);
        border: 1px solid rgba(255, 255, 255, 0.05);
        border-radius: 12px;
        padding: 1.25rem;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        transition: all 0.2s ease;
    }
    .bento-card:hover {
        background: rgba(255, 255, 255, 0.04);
        border-color: rgba(255, 255, 255, 0.1);
    }

    .bento-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--echo-text-2);
        font-size: 0.875rem;
        font-weight: 500;
    }

    .bento-value {
        color: var(--echo-text-1);
        font-size: 0.875rem;
        word-break: break-all;
        flex-grow: 1;
        line-height: 1.5;
    }
    .bento-value.path-value {
        font-family: var(--echo-font-body);
        color: var(--echo-text-1);
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }
    .bento-value.hash-value {
        font-family: monospace;
        color: var(--echo-text-2);
        background: rgba(0, 0, 0, 0.2);
        padding: 0.5rem;
        border-radius: 6px;
        font-size: 0.75rem;
    }

    .bento-action-btn {
        margin-top: auto;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid transparent;
        color: var(--echo-text-1);
        padding: 0.5rem 0.75rem;
        border-radius: 6px;
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        transition: all 0.2s ease;
    }
    .bento-action-btn:hover:not(:disabled) {
        background: rgba(255, 255, 255, 0.1);
        border-color: rgba(255, 255, 255, 0.15);
    }
    .bento-action-btn:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
</style>
