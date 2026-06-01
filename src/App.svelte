<script lang="ts">
  import { store } from './lib/stores.svelte';
  import { quitApp } from './lib/api';
  import DeviceList from './lib/DeviceList.svelte';
  import AddDevice from './lib/AddDevice.svelte';
  import ScanView from './lib/ScanView.svelte';
  import Settings from './lib/Settings.svelte';
  import { ask } from '@tauri-apps/plugin-dialog';

  const page = $derived(store.page);

  async function handleClose() {
    const confirmed = await ask('Are you sure you want to quit ADB Bar?', {
      title: 'Quit ADB Bar',
      kind: 'warning',
      okLabel: 'Quit',
      cancelLabel: 'Cancel',
    });
    if (confirmed) {
      await quitApp();
    }
  }
</script>

<div class="app-container">
  <div class="popover-arrow"></div>
  <div class="titlebar">
    <span class="titlebar-text">ADB Bar</span>
    <button class="close-btn" onclick={handleClose} title="Quit">
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path d="M2 2L10 10M10 2L2 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
    </button>
  </div>

  <div class="content">
    {#if page === 'main'}
      <DeviceList />
    {:else if page === 'addDevice'}
      <AddDevice />
    {:else if page === 'scan'}
      <ScanView />
    {:else if page === 'settings'}
      <Settings />
    {/if}
  </div>
</div>

<style>
  .app-container {
    width: 100%;
    height: calc(100vh - 10px);
    margin-top: 10px;
    position: relative;
    display: flex;
    flex-direction: column;
    background: rgba(30, 30, 30, 0.92);
    backdrop-filter: blur(40px);
    -webkit-backdrop-filter: blur(40px);
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    font-size: 13px;
    overflow: visible;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 0 0.5px rgba(255, 255, 255, 0.05);
  }

  .popover-arrow {
    position: absolute;
    top: 0;
    left: 50%;
    width: 20px;
    height: 20px;
    background: rgba(30, 30, 30, 0.96);
    border-left: 1px solid rgba(255, 255, 255, 0.08);
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    transform: translate(-50%, -50%) rotate(45deg);
    pointer-events: none;
    z-index: 1;
  }

  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 6px;
    -webkit-user-select: none;
    user-select: none;
    cursor: default;
    position: relative;
    z-index: 2;
  }

  .titlebar-text {
    font-size: 12px;
    font-weight: 600;
    color: #888;
    letter-spacing: 0.3px;
  }

  .close-btn {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.06);
    color: #888;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    padding: 0;
  }

  .close-btn:hover {
    background: rgba(255, 80, 80, 0.3);
    border-color: rgba(255, 80, 80, 0.4);
    color: #ff6b6b;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
