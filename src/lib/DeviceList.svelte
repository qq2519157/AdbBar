<script lang="ts">
  import { slide } from 'svelte/transition';
  import { store } from './stores.svelte';
  import { getDevices, refreshAll } from './api';
  import DeviceRow from './DeviceRow.svelte';

  let loading = $state(false);

  const devices = $derived(store.devices);
  const statusMessage = $derived(store.statusMessage);
  const isRefreshing = $derived(store.isRefreshing);

  async function loadDevices() {
    loading = true;
    try {
      store.devices = await getDevices();
    } catch (e) {
      store.showStatus('Failed to load devices');
    } finally {
      loading = false;
    }
  }

  async function handleRefresh() {
    store.isRefreshing = true;
    try {
      store.devices = await refreshAll();
      store.showStatus('Refreshed');
    } catch (e) {
      store.showStatus('Refresh failed');
    } finally {
      store.isRefreshing = false;
    }
  }

  $effect(() => {
    loadDevices();
  });
</script>

<div class="device-list">
  <header class="header">
    <h1 class="title">ADB Devices</h1>
    <div class="header-actions">
      <button
        class="icon-btn"
        onclick={handleRefresh}
        disabled={isRefreshing}
        title="Refresh"
      >
        <svg class="icon {isRefreshing ? 'spinning' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 2v6h-6" />
          <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
          <path d="M3 22v-6h6" />
          <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
        </svg>
      </button>
      <button
        class="icon-btn"
        onclick={() => store.navigate('settings')}
        title="Settings"
      >
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    </div>
  </header>

  {#if statusMessage}
    <div class="status-bar" transition:slide={{ duration: 150 }}>
      {statusMessage}
    </div>
  {/if}

  <div class="device-scroll">
    {#if loading && devices.length === 0}
      <div class="empty-state">
        <p class="empty-text">Loading devices...</p>
      </div>
    {:else if devices.length === 0}
      <div class="empty-state">
        <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="5" y="2" width="14" height="20" rx="2" />
          <line x1="12" y1="18" x2="12" y2="18.01" />
        </svg>
        <p class="empty-text">No devices found</p>
        <p class="empty-sub">Add a device or scan your network</p>
      </div>
    {:else}
      {#each devices as device (device.id)}
        <DeviceRow {device} />
      {/each}
    {/if}
  </div>

  <footer class="footer">
    <button class="glass-btn" onclick={() => store.navigate('addDevice')}>
      <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="5" x2="12" y2="19" />
        <line x1="5" y1="12" x2="19" y2="12" />
      </svg>
      Add Device
    </button>
    <button class="glass-btn" onclick={() => store.navigate('scan')}>
      <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="12" x2="16" y2="12" />
      </svg>
      Scan
    </button>
  </footer>
</div>

<style>
  .device-list {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
    color: #fff;
  }

  .header-actions {
    display: flex;
    gap: 4px;
  }

  .icon-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    color: #b0b0b0;
    cursor: pointer;
    padding: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
  }

  .icon-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon {
    width: 16px;
    height: 16px;
  }

  .spinning {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .status-bar {
    padding: 6px 14px;
    font-size: 11px;
    color: #8cb4ff;
    background: rgba(100, 160, 255, 0.08);
    border-bottom: 1px solid rgba(100, 160, 255, 0.1);
    text-align: center;
  }

  .device-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 6px 8px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 8px;
    opacity: 0.5;
  }

  .empty-icon {
    width: 40px;
    height: 40px;
    color: #666;
  }

  .empty-text {
    font-size: 13px;
    margin: 0;
    color: #999;
  }

  .empty-sub {
    font-size: 11px;
    margin: 0;
    color: #666;
  }

  .footer {
    display: flex;
    gap: 8px;
    padding: 10px 12px 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .glass-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .glass-btn:hover {
    background: rgba(255, 255, 255, 0.14);
    border-color: rgba(255, 255, 255, 0.18);
    color: #fff;
  }

  .glass-btn:active {
    transform: scale(0.97);
  }

  .btn-icon {
    width: 14px;
    height: 14px;
  }
</style>
