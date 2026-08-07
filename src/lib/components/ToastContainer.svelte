<script lang="ts">
    import { toastStore } from "$lib/stores/toast.svelte";
    import { X } from "phosphor-svelte";
    import { fly } from "svelte/transition";
    import { flip } from "svelte/animate";
</script>

{#if toastStore.toasts.length > 0}
    <div class="toast-container">
        {#each toastStore.toasts as toast (toast.id)}
            <div class="toast toast-{toast.type}" role="alert" animate:flip={{ duration: 300 }} transition:fly={{ x: 50, duration: 300 }}>
                <span class="toast-message">{toast.message}</span>
                <button class="toast-dismiss" onclick={() => toastStore.dismiss(toast.id)} aria-label="Dismiss">
                    <X size={16} weight="bold" />
                </button>
            </div>
        {/each}
    </div>
{/if}

<style>
    .toast-container {
        position: fixed;
        bottom: 120px; /* Above the PlayerBar */
        right: 1.5rem;
        z-index: 200;
        display: flex;
        flex-direction: column-reverse;
        gap: 0.5rem;
        pointer-events: none;
    }

    .toast {
        pointer-events: all;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.75rem 1rem;
        border-radius: 10px;
        font-family: var(--echo-font-body);
        font-size: 0.9rem;
        color: var(--echo-text-1);
        background: var(--echo-surface);
        border: 1px solid var(--echo-border);
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
        max-width: 380px;
    }

    .toast-info {
        border-left: 3px solid var(--echo-primary);
    }
    .toast-success {
        border-left: 3px solid #10b981;
    }
    .toast-error {
        border-left: 3px solid #ef4444;
    }

    .toast-message {
        flex: 1;
    }

    .toast-dismiss {
        background: transparent;
        border: none;
        color: var(--echo-text-2);
        padding: 0.25rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        border-radius: 4px;
        transition: color 0.15s;
    }
    .toast-dismiss:hover {
        color: #fff;
        background: rgba(255, 255, 255, 0.1);
        transform: none;
    }

</style>
