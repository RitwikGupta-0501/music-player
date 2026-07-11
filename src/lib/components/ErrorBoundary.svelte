<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
  let error: string | null = $state(null);
  let errorCount = $state(0);

  let hasError = $derived(error !== null);

  function resetError() {
    error = null;
  }

  function handleError(e: Event) {
    if (e instanceof ErrorEvent) {
      error = e.message || String(e);
    } else {
      error = String(e);
    }
    errorCount++;
    console.error('ErrorBoundary caught:', error);
  }
</script>

<svelte:window on:error={handleError} />

{#if hasError}
  <div class="error-container">
    <div class="error-content">
      <h2>Something went wrong</h2>
      <p class="error-message">{error}</p>
      <div class="error-actions">
        <button onclick={resetError} class="btn btn-primary">
          Try again
        </button>
      </div>
      {#if errorCount > 3}
        <p class="error-hint">
          Multiple errors detected. Consider restarting the application.
        </p>
      {/if}
    </div>
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .error-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
    background: #0a0a0a;
  }

  .error-content {
    background: #1a1a1a;
    border: 1px solid #333;
    border-radius: 0.5rem;
    padding: 2rem;
    max-width: 500px;
    text-align: center;
  }

  h2 {
    color: #ff6b6b;
    margin: 0 0 1rem;
    font-size: 1.5rem;
  }

  .error-message {
    color: #999;
    margin: 0 0 1.5rem;
    font-family: monospace;
    font-size: 0.875rem;
    word-break: break-word;
  }

  .error-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: center;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.25rem;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background: #007bff;
    color: white;
  }

  .btn-primary:hover {
    background: #0056b3;
  }

  .error-hint {
    color: #ff9500;
    margin-top: 1rem;
    font-size: 0.875rem;
  }
</style>
