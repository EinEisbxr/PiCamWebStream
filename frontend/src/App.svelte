<script lang="ts">
  import { onMount } from 'svelte';
  import type { BackendConfig } from './lib/types';
  import { fetchConfig, fetchHealth, streamUrl } from './lib/api';

  let config: BackendConfig | null = null;
  let error: string | null = null;
  let loading = true;
  let health: 'ok' | 'error' | 'unknown' = 'unknown';
  let forceReloadToken = 0;
  $: healthIndicatorClass =
    health === 'ok'
      ? 'bg-emerald-400'
      : health === 'error'
        ? 'bg-rose-500'
        : 'bg-amber-400';

  async function refreshConfig() {
    try {
      loading = true;
      config = await fetchConfig();
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load config';
    } finally {
      loading = false;
    }
  }

  async function checkHealth() {
    try {
      await fetchHealth();
      health = 'ok';
    } catch (err) {
      console.error(err);
      health = 'error';
    }
  }

  function reloadStream() {
    forceReloadToken = Date.now();
  }

  onMount(() => {
    refreshConfig();
    checkHealth();
    const healthInterval = setInterval(checkHealth, 10000);
    return () => clearInterval(healthInterval);
  });
</script>

<main class="min-h-screen px-4 pb-16">
  <section class="mx-auto flex max-w-4xl flex-col gap-8 pt-10">
    <header class="flex flex-col gap-4 rounded-xl border border-slate-800 bg-slate-950/80 p-6 shadow-xl shadow-black/40 backdrop-blur">
      <div>
        <h1 class="text-3xl font-semibold">PiCam Web Stream</h1>
        <p class="mt-1 text-sm text-slate-300">
          Securely stream your Raspberry Pi camera feed through the Rust backend.
        </p>
      </div>
      <div class="flex flex-wrap items-center gap-3 text-sm text-slate-300">
        <span class="inline-flex items-center gap-2 rounded-full bg-slate-800/80 px-3 py-1">
          <span class={`h-2 w-2 rounded-full ${healthIndicatorClass}`}></span>
          {#if health === 'ok'}
            Backend healthy
          {:else if health === 'error'}
            Backend unreachable
          {:else}
            Checking backend…
          {/if}
        </span>
        {#if config}
          <span class="inline-flex items-center gap-2 rounded-full bg-slate-800/80 px-3 py-1">
            <span class="text-xs uppercase tracking-wide text-slate-400">FPS</span>
            <span class="font-semibold">{config.frame_rate}</span>
          </span>
          <span class="inline-flex items-center gap-2 rounded-full bg-slate-800/80 px-3 py-1">
            <span class="text-xs uppercase tracking-wide text-slate-400">Resolution</span>
            <span class="font-semibold">{config.resolution_width} × {config.resolution_height}</span>
          </span>
          {#if config.camera_device}
            <span class="inline-flex items-center gap-2 rounded-full bg-slate-800/80 px-3 py-1">
              <span class="text-xs uppercase tracking-wide text-slate-400">Device</span>
              <span class="font-semibold">{config.camera_device}</span>
            </span>
          {/if}
        {/if}
      </div>
    </header>

    <section class="rounded-xl border border-slate-800 bg-slate-950/60 p-4 shadow-xl shadow-black/30 backdrop-blur">
      {#if loading}
        <div class="flex min-h-[320px] items-center justify-center text-slate-400">
          Loading configuration…
        </div>
      {:else if error}
        <div class="flex min-h-[320px] flex-col items-center justify-center gap-3 text-center text-slate-300">
          <p class="text-lg font-medium text-rose-300">{error}</p>
          <button class="rounded-lg bg-rose-600 px-4 py-2 text-sm font-semibold text-white shadow-lg shadow-rose-900/40 transition hover:bg-rose-500" on:click={refreshConfig}>
            Try again
          </button>
        </div>
      {:else}
        <div class="flex flex-col gap-4">
          <div class="relative overflow-hidden rounded-lg border border-slate-800 bg-black/80 shadow-inner">
            <img
              src={`${streamUrl()}?token=${forceReloadToken}`}
              alt="Pi camera stream"
              class="mx-auto block h-auto w-full max-h-[70vh] object-contain"
            />
          </div>
          <div class="flex flex-wrap items-center justify-between gap-4 text-sm text-slate-300">
            <span>MJPEG stream powered by Rust backend.</span>
            <div class="flex gap-2">
              <button class="rounded-lg border border-slate-700 px-3 py-1 transition hover:border-slate-500" on:click={reloadStream}>
                Reload stream
              </button>
              <button class="rounded-lg border border-slate-700 px-3 py-1 transition hover:border-slate-500" on:click={refreshConfig}>
                Refresh config
              </button>
            </div>
          </div>
        </div>
      {/if}
    </section>

    <section class="grid gap-4 rounded-xl border border-slate-800 bg-slate-950/40 p-6 text-sm text-slate-400">
      <h2 class="text-lg font-semibold text-slate-200">Quick tips</h2>
      <ul class="list-disc space-y-2 pl-6">
        <li>Set <code>STREAM_USER</code> and <code>STREAM_PASS</code> on the backend once authentication is implemented.</li>
        <li>Expose the backend via HTTPS when deploying publicly.</li>
        <li>Use docker compose to launch both frontend and backend together.</li>
      </ul>
    </section>
  </section>
</main>

<style>
  main {
    background: radial-gradient(circle at top, rgba(59, 130, 246, 0.15), transparent 60%),
      radial-gradient(circle at bottom, rgba(16, 185, 129, 0.1), transparent 65%),
      #020617;
  }
</style>
