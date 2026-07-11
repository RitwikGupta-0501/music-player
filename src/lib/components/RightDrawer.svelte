<script lang="ts">
    import { fade } from "svelte/transition";
    import { X } from "phosphor-svelte";
    
    let { 
        title, 
        isOpen,
        onClose,
        children
    } = $props<{ 
        title: string;
        isOpen: boolean;
        onClose: () => void;
        children?: import('svelte').Snippet;
    }>();
</script>

    <aside class="right-drawer" class:is-open={isOpen}>
        <div class="drawer-content">
            <div class="drawer-header">
                <h2 class="drawer-title">{title}</h2>
                <button 
                    class="close-btn"
                    onclick={onClose}
                    aria-label="Close"
                >
                    <X size={20} weight="bold" />
                </button>
            </div>
            
            <div class="drawer-body">
                {#key title}
                    <div transition:fade={{ duration: 150 }} style="height: 100%;">
                        {@render children?.()}
                    </div>
                {/key}
            </div>
        </div>
    </aside>

<style>
    .right-drawer {
        width: 0px;
        height: 100%;
        background-color: #08080a;
        border-left: 1px solid transparent;
        flex-shrink: 0;
        position: relative;
        z-index: 20;
        overflow: hidden;
    }
    .right-drawer.is-open {
        width: 400px;
        border-left: 1px solid rgba(255, 255, 255, 0.05); /* border-white/5 */
    }

    .drawer-content {
        width: 400px; /* Force internal width so it doesn't squish during transition */
        height: 100%;
        display: flex;
        flex-direction: column;
    }

    .drawer-header {
        padding: 1.5rem; /* px-6 py-6 */
        display: flex;
        align-items: center;
        justify-content: space-between;
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        flex-shrink: 0;
    }

    .drawer-title {
        font-size: 1.125rem; /* text-lg */
        font-family: var(--echo-font-body);
        font-weight: 500;
        letter-spacing: 0.025em; /* tracking-wide */
        text-transform: uppercase;
        color: var(--echo-primary-dark);
        margin: 0;
    }

    .close-btn {
        width: 32px; /* w-8 */
        height: 32px; /* h-8 */
        border-radius: 9999px; /* rounded-full */
        background: transparent;
        border: none;
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--echo-text-2); /* text-muted */
        cursor: pointer;
        transition: all 0.2s;
        padding: 0;
    }

    .close-btn:hover {
        background-color: rgba(255, 255, 255, 0.05); /* hover:bg-white/5 */
        color: var(--echo-text-1); /* hover:text-text-main */
    }

    .drawer-body {
        flex: 1;
        overflow-y: auto;
        position: relative;
        /* Custom scrollbar to hide by default */
        scrollbar-width: none;
        -ms-overflow-style: none;
    }
    
    .drawer-body::-webkit-scrollbar {
        display: none;
    }
</style>
