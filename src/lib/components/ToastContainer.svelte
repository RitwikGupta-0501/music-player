<script lang="ts">
    import { toastStore } from "$lib/stores/toast.svelte";
    import { X } from "phosphor-svelte";
</script>

{#if toastStore.toasts.length > 0}
    <div class="toast-container">
        {#each toastStore.toasts as toast (toast.id)}
            <div class="toast toast-{toast.type}" role="alert">
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
        border-radius: 8px;
        font-family: var(--font-body);
        font-size: 0.9rem;
        color: var(--color-chalk);
        background: rgba(31, 40, 51, 0.9);
        backdrop-filter: blur(16px);
        -webkit-backdrop-filter: blur(16px);
        border: 1px solid var(--glass-border);
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
        animation: toast-slide-in 0.3s ease-out;
        max-width: 380px;
    }

    .toast-info {
        border-left: 3px solid var(--color-cyan);
    }
    .toast-success {
        border-left: 3px solid var(--color-teal);
    }
    .toast-error {
        border-left: 3px solid var(--color-danger);
    }

    .toast-message {
        flex: 1;
    }

    .toast-dismiss {
        background: transparent;
        border: none;
        color: var(--color-chalk-muted);
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

    @keyframes toast-slide-in {
        from {
            opacity: 0;
            transform: translateX(1rem);
        }
        to {
            opacity: 1;
            transform: translateX(0);
        }
    }
</style>
