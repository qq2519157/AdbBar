<script lang="ts">
  import { store } from './stores.svelte';
  import {
    getDeviceProps,
    connectDevice,
    disconnectDevice,
    launchScrcpy,
    refreshAll,
  } from './api';
  import { getErrorMessage } from './errors';
  import { t } from './i18n';
  import { untrack } from 'svelte';

  let props = $state<Record<string, string> | null>(null);
  let error = $state('');
  let loading = $state(true);
  let actionBusy = $state(false);

  const device = $derived(
    store.devices.find((d) => d.id === store.selectedDeviceId) ?? null
  );
  const address = $derived(device ? `${device.ip_address}:${device.port}` : '');

  const summaryKeys: [string, string][] = [
    ['ro.product.model', 'deviceDetail.model'],
    ['ro.product.brand', 'deviceDetail.brand'],
    ['ro.build.version.release', 'deviceDetail.android'],
    ['ro.build.version.sdk', 'deviceDetail.sdk'],
    ['ro.product.cpu.abi', 'deviceDetail.abi'],
    ['ro.serialno', 'deviceDetail.serial'],
  ];

  const summary = $derived.by(() => {
    const rows: { label: string; value: string }[] = [];
    for (const [propKey, labelKey] of summaryKeys) {
      const value = props?.[propKey];
      if (value) {
        rows.push({ label: t(labelKey), value });
      }
    }
    return rows;
  });

  const allProps = $derived(Object.entries(props ?? {}));

  async function load(id: string | null) {
    if (!id) {
      return;
    }
    const dev = store.devices.find((d) => d.id === id);
    if (!dev) {
      return;
    }
    const addr = `${dev.ip_address}:${dev.port}`;
    loading = true;
    error = '';
    props = null;
    try {
      props = await getDeviceProps(addr);
    } catch (e) {
      error = getErrorMessage(e, t('deviceDetail.loadFailed'));
    } finally {
      loading = false;
    }
  }

  function handleBack() {
    store.selectedDeviceId = null;
    store.navigate('main');
  }

  async function handleConnect() {
    if (!address || actionBusy) {
      return;
    }
    actionBusy = true;
    try {
      await connectDevice(address);
      store.devices = await refreshAll();
      store.showStatus(t('deviceRow.connected'));
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('deviceRow.connectionFailed')));
    } finally {
      actionBusy = false;
    }
  }

  async function handleDisconnect() {
    if (!address || actionBusy) {
      return;
    }
    actionBusy = true;
    try {
      await disconnectDevice(address);
      store.devices = await refreshAll();
      store.showStatus(t('deviceRow.disconnected'));
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('deviceRow.disconnectFailed')));
    } finally {
      actionBusy = false;
    }
  }

  async function handleScrcpy() {
    if (!address || actionBusy) {
      return;
    }
    actionBusy = true;
    try {
      await launchScrcpy(address);
      store.showStatus(t('deviceRow.scrcpyLaunched'));
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('deviceRow.scrcpyFailed')));
    } finally {
      actionBusy = false;
    }
  }

  // Reload only when a different device is opened; background device-list
  // refreshes must not re-trigger the fetch.
  $effect(() => {
    const id = store.selectedDeviceId;
    untrack(() => load(id));
  });
</script>

<div class="device-detail">
  <header class="page-header">
    <button class="back-btn" onclick={handleBack} title="Back">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <h1 class="page-title">{device ? device.name : t('deviceDetail.title')}</h1>
  </header>

  {#if device}
    <div class="detail-content">
      <div class="address-row">
        <span class="address">{address}</span>
        <span class="status">{device.status}</span>
      </div>

      <div class="actions-row">
        {#if device.status === 'connected'}
          <button class="glass-btn" onclick={handleDisconnect} disabled={actionBusy}>
            {t('deviceRow.disconnect')}
          </button>
          <button class="glass-btn" onclick={handleScrcpy} disabled={actionBusy}>
            {t('deviceRow.scrcpy')}
          </button>
        {:else}
          <button class="glass-btn" onclick={handleConnect} disabled={actionBusy}>
            {t('deviceRow.connect')}
          </button>
        {/if}
      </div>

      {#if loading}
        <div class="state-text">{t('deviceDetail.loading')}</div>
      {:else if error}
        <p class="error-text">{error}</p>
      {:else if summary.length > 0}
        <div class="summary">
          {#each summary as row (row.label)}
            <div class="summary-row">
              <span class="summary-label">{row.label}</span>
              <span class="summary-value">{row.value}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#if !loading && !error && allProps.length > 0}
        <div class="props-header">{t('deviceDetail.allProps')} ({allProps.length})</div>
        <div class="props-list">
          {#each allProps as [key, value] (key)}
            <div class="prop-row">
              <span class="prop-key">{key}</span>
              <span class="prop-value">{value}</span>
            </div>
          {/each}
        </div>
      {:else if !loading && !error && props}
        <div class="state-text">{t('deviceDetail.empty')}</div>
      {/if}
    </div>
  {:else}
    <div class="detail-content">
      <div class="state-text">{t('deviceDetail.loadFailed')}</div>
    </div>
  {/if}
</div>

<style>
  .device-detail {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 10px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .back-btn {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    color: #b0b0b0;
    cursor: pointer;
    padding: 5px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .back-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
  }

  .icon {
    width: 16px;
    height: 16px;
  }

  .page-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
    color: #fff;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .detail-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px 14px;
  }

  .address-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    margin-bottom: 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
  }

  .address {
    font-size: 12px;
    color: #ccc;
    font-family: 'SF Mono', 'Menlo', monospace;
  }

  .status {
    font-size: 10px;
    color: #8cb4ff;
  }

  .actions-row {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .glass-btn {
    flex: 1;
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

  .glass-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .summary {
    padding: 10px 12px;
    margin-bottom: 12px;
    background: rgba(100, 180, 255, 0.06);
    border: 1px solid rgba(100, 180, 255, 0.12);
    border-radius: 8px;
  }

  .summary-row {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 4px 0;
  }

  .summary-label {
    font-size: 12px;
    color: #999;
    flex-shrink: 0;
  }

  .summary-value {
    font-size: 12px;
    color: #e8e8e8;
    font-weight: 500;
    text-align: right;
    word-break: break-all;
  }

  .props-header {
    font-size: 10px;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .props-list {
    display: flex;
    flex-direction: column;
  }

  .prop-row {
    display: flex;
    flex-direction: column;
    padding: 6px 10px;
    border-radius: 6px;
  }

  .prop-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .prop-key {
    font-size: 11px;
    color: #8cb4ff;
    font-family: 'SF Mono', 'Menlo', monospace;
    word-break: break-all;
  }

  .prop-value {
    font-size: 11px;
    color: #aaa;
    font-family: 'SF Mono', 'Menlo', monospace;
    word-break: break-all;
  }

  .state-text {
    padding: 24px 0;
    text-align: center;
    color: #777;
    font-size: 12px;
  }

  .error-text {
    margin: 0;
    padding: 8px 10px;
    color: #ff8a8a;
    background: rgba(255, 100, 100, 0.08);
    border-radius: 8px;
    font-size: 12px;
    word-break: break-all;
  }
</style>
