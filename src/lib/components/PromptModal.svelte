<script lang="ts">
    import { onMount } from "svelte";

    let { title = "Enter value", defaultValue = "", onSubmit, onClose } = $props<{
        title?: string;
        defaultValue?: string;
        onSubmit: (val: string) => void;
        onClose: () => void;
    }>();

    let value = $state(defaultValue);
    let inputRef = $state<HTMLInputElement | null>(null);

    onMount(() => {
        if (inputRef) {
            inputRef.focus();
            inputRef.select();
        }
    });

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            e.preventDefault();
            handleSubmit();
        } else if (e.key === "Escape") {
            onClose();
        }
    }

    function handleSubmit() {
        if (value.trim()) {
            onSubmit(value.trim());
        }
    }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-overlay" onclick={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="glass-panel modal-content" onclick={(e) => e.stopPropagation()} role="dialog">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem;">
            <h2>{title}</h2>
            <button class="ghost" style="padding: 0.2rem 0.5rem;" onclick={onClose}>✕</button>
        </div>
        
        <input 
            bind:this={inputRef}
            type="text" 
            bind:value={value} 
            onkeydown={handleKeydown}
            spellcheck="false"
            placeholder="Name..."
            style="width: 100%; margin-bottom: 1.5rem;" 
        />
        
        <div style="display: flex; justify-content: flex-end; gap: 0.5rem;">
            <button class="ghost" onclick={onClose}>Cancel</button>
            <button class="primary" onclick={handleSubmit} disabled={!value.trim()}>Create</button>
        </div>
    </div>
</div>

<style>
    .modal-overlay {
        position: fixed;
        top: 0;
        left: var(--sidebar-w, 80px);
        right: var(--drawer-w, 0px);
        bottom: 0;
        background: rgba(0,0,0,0.7);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 100;
        backdrop-filter: blur(4px);
    }
    .modal-content {
        padding: 2.5rem;
        width: 100%;
        max-width: 400px;
        display: flex;
        flex-direction: column;
        background: var(--echo-surface);
        border: 1px solid var(--echo-border);
        border-radius: 1.5rem;
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
    }
</style>
