<script lang="ts">
  import type { AdbDevice } from './types';
  import { store } from './stores';
  import {
    connectDevice,
    disconnectDevice,
    removeDevice,
    openShell,
    launchScrcpy,
    takeScreenshot,
    installApk,
    getDevices,
  } from './api';
  import { open } from '@tauri-apps/plugin-dialog';

  let { device }: { device: AdbDevice } = $props();

  let menuOpen = $state(false);
  let busy = $state(false);

  const address = $derived(`${device.ip_address}:${device.port}`);

  const statusColor = $derived.by(() => {
    switch (device.status) {
      case 'connected': return '#4caf50';
      case 'connecting': return '#ffca28';
      case 'unauthorized': return '#ff9800';
      case 'offline': return '#f44336';
      default: return '#666';
    }
  });

  const statusLabel = $derived.by(() => {
    switch (device.status) {
      case 'connected': return 'Connected';
      case 'connecting': return 'Connecting';
      case 'unauthorized': return 'Unauthorized';
      case 'offline': return 'Offline';
      default: return 'Disconnected';
    }
  });

  async function handleConnect() {
    busy = true;
    try {
      const result = await connectDevice(address);
      store.showStatus(result.message);
      store.devices = await getDevices();
    } catch (e) {
      store.showStatus('Connection failed');
    } finally {
      busy = false;
    }
  }

  async function handleDisconnect() {
    busy = true;
    menuOpen = false;
    try {
      const result = await disconnectDevice(address);
      store.showStatus(result.message);
      store.devices = await getDevices();
    } catch (e) {
      store.showStatus('Disconnect failed');
    } finally {
      busy = false;
    }
  }

  async function handleDelete() {
    menuOpen = false;
    try {
      await removeDevice(device.id);
      store.devices = await getDevices();
      store.showStatus('Device removed');
    } catch (e) {
      store.showStatus('Failed to remove device');
    }
  }

  async function handleShell() {
    menuOpen = false;
    try {
      await openShell(address);
      store.showStatus('Shell opened');
    } catch (e) {
      store.showStatus('Failed to open shell');
    }
  }

  async function handleScrcpy() {
    menuOpen = false;
    try {
      await launchScrcpy(address);
      store.showStatus('Scrcpy launched');
    } catch (e) {
      store.showStatus('Failed to launch scrcpy');
    }
  }

  async function handleScreenshot() {
    menuOpen = false;
    busy = true;
    try {
      const result = await takeScreenshot(address);
      store.showStatus(result.message);
    } catch (e) {
      store.showStatus('Screenshot failed');
    } finally {
      busy = false;
    }
  }

  async function handleInstallApk() {
    menuOpen = false;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'APK', extensions: ['apk'] }],
      });
      if (selected) {
        const filePath = typeof selected === 'string' ? selected : selected;
        busy = true;
        const result = await installApk(address, filePath as string);
        store.showStatus(result.message);
      }
    } catch (e) {
      store.showStatus('APK install failed');
    } finally {
      busy = false;
    }
  }

  function toggleMenu() {
    menuOpen = !menuOpen;
  }

  function closeMenu() {
    menuOpen = false;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="device-row-wrapper" tabindex="-1" onblur={closeMenu}>
  <div class="device-row">
    <div class="device-info">
      <div class="status-dot" style="background: {statusColor};"></div>
      <div class="device-text">
        <span class="device-name">{device.name}</span>
        <span class="device-ip">{address}</span>
      </div>
    </div>

    <div class="device-actions">
      {#if device.status === 'disconnected' || device.status === 'offline'}
        <button
          class="connect-btn"
          onclick={handleConnect}
          disabled={busy}
        >
          Connect
        </button>
      {:else if device.status === 'connected'}
        <button
          class="connect-btn disconnect"
          onclick={handleDisconnect}
          disabled={busy}
        >
          Disconnect
        </button>
      {/if}

      <button class="menu-btn" onclick={toggleMenu} title="Actions">
        &#8942;
      </button>
    </div>
  </div>

  {#if menuOpen}
    <div class="action-menu">
      {#if device.status === 'connected'}
        <button class="menu-item" onclick={handleShell}>
          <svg class="mi-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          Shell
        </button>
        <button class="menu-item" onclick={handleScrcpy}>
          <svg class="mi-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="2" y="3" width="20" height="14" rx="2" />
            <line x1="8" y1="21" x2="16" y2="21" />
            <line x1="12" y1="17" x2="12" y2="21" />
          </svg>
          Scrcpy
        </button>
        <button class="menu-item" onclick={handleScreenshot}>
          <svg class="mi-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
            <circle cx="12" cy="13" r="4" />
          </svg>
          Screenshot
        </button>
        <button class="menu-item" onclick={handleInstallApk}>
          <svg class="mi-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
          Install APK
        </button>
        <div class="menu-divider"></div>
      {/if}
      <button class="menu-item danger" onclick={handleDelete}>
        <svg class="mi-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6" />
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
        </svg>
        Remove Device
      </button>
    </div>
  {/if}
</div>

<style>
  .device-row-wrapper {
    position: relative;
    outline: none;
  }

  .device-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-radius: 8px;
    transition: background 0.12s ease;
  }

  .device-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .device-info {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 4px currentColor;
  }

  .device-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .device-name {
    font-size: 13px;
    font-weight: 500;
    color: #e8e8e8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .device-ip {
    font-size: 11px;
    color: #888;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .device-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .connect-btn {
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 500;
    border: 1px solid rgba(100, 180, 255, 0.3);
    border-radius: 6px;
    background: rgba(100, 180, 255, 0.1);
    color: #8cb4ff;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .connect-btn:hover {
    background: rgba(100, 180, 255, 0.2);
    border-color: rgba(100, 180, 255, 0.4);
  }

  .connect-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .connect-btn.disconnect {
    border-color: rgba(255, 100, 100, 0.3);
    background: rgba(255, 100, 100, 0.1);
    color: #ff8a8a;
  }

  .connect-btn.disconnect:hover {
    background: rgba(255, 100, 100, 0.2);
    border-color: rgba(255, 100, 100, 0.4);
  }

  .menu-btn {
    background: transparent;
    border: none;
    color: #888;
    cursor: pointer;
    padding: 4px 6px;
    font-size: 16px;
    line-height: 1;
    border-radius: 4px;
    transition: all 0.12s ease;
  }

  .menu-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #ccc;
  }

  .action-menu {
    position: absolute;
    right: 8px;
    top: calc(100% - 4px);
    z-index: 100;
    background: rgba(45, 45, 45, 0.95);
    backdrop-filter: blur(16px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 4px;
    min-width: 160px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    animation: menuIn 0.1s ease-out;
  }

  @keyframes menuIn {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    font-size: 12px;
    color: #ccc;
    background: transparent;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .menu-item:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }

  .menu-item.danger {
    color: #ff8a8a;
  }

  .menu-item.danger:hover {
    background: rgba(255, 100, 100, 0.12);
    color: #ff6b6b;
  }

  .mi-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  .menu-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.08);
    margin: 4px 8px;
  }
</style>
