<script lang="ts">
    import { toastStore } from "$lib/stores/toast.svelte";
    import { X, Info, CheckCircle, WarningCircle } from "phosphor-svelte";
    import { fly } from "svelte/transition";
    import { flip } from "svelte/animate";
    import { quintOut } from "svelte/easing";
</script>

<div class="toast-container">
    {#each toastStore.toasts as toast (toast.id)}
        <div class="toast" role="alert" animate:flip={{ duration: 400, easing: quintOut }} transition:fly={{ y: 20, duration: 400, easing: quintOut }}>
            <div class="toast-icon toast-{toast.type}">
                {#if toast.type === 'success'}
                    <CheckCircle size={20} weight="fill" />
                {:else if toast.type === 'error'}
                    <WarningCircle size={20} weight="fill" />
                {:else}
                    <Info size={20} weight="fill" />
                {/if}
            </div>
            <span class="toast-message">{toast.message}</span>
            <button class="toast-dismiss" onclick={() => toastStore.dismiss(toast.id)} aria-label="Dismiss">
                <X size={16} weight="bold" />
            </button>
        </div>
    {/each}
</div>

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
        gap: 0.875rem;
        padding: 0.875rem 1rem;
        border-radius: 12px;
        font-family: var(--echo-font-body);
        font-size: 0.95rem;
        font-weight: 500;
        color: var(--echo-text-1);
        background: rgba(24, 24, 27, 0.65);
        backdrop-filter: blur(20px);
        -webkit-backdrop-filter: blur(20px);
        border: 1px solid rgba(255, 255, 255, 0.08);
        box-shadow: 0 10px 40px -10px rgba(0, 0, 0, 0.7), inset 0 1px 0 rgba(255, 255, 255, 0.05);
        max-width: 380px;
        will-change: transform, opacity;
    }

    .toast-icon {
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .toast-info {
        color: var(--echo-primary);
    }
    .toast-success {
        color: #10b981; /* emerald-500 */
    }
    .toast-error {
        color: #ef4444; /* red-500 */
    }

    .toast-message {
        flex: 1;
        letter-spacing: 0.01em;
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
