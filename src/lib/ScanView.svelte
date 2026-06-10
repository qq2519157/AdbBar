<script lang="ts">
  import { store } from './stores.svelte';
  import { scanNetwork, addDevice, getDevices } from './api';
  import { listen } from './api';
  import { getErrorMessage } from './errors';
  import { t } from './i18n';
  import type { ScanProgress, ScanResult } from './types';

  let unlisten: (() => void) | null = $state(null);
  let portInput = $state(String(store.scanSession.port));
  let localError = $state<string | null>(null);
  let startedOnMount = $state(false);

  const scanSession = $derived(store.scanSession);
  const isScanning = $derived(store.isScanning);
  const devices = $derived(store.devices);
  const visibleResults = $derived.by(() => {
    if (scanSession.results.length > 0) {
      return scanSession.results;
    }
    return scanSession.progress.found;
  });
  const hasFinished = $derived(scanSession.completedAt !== null && !isScanning);
  const progressPercent = $derived.by(() => {
    if (scanSession.progress.total <= 0) {
      return 0;
    }
    return Math.min(
      100,
      Math.round((scanSession.progress.scanned / scanSession.progress.total) * 100)
    );
  });
  const elapsedLabel = $derived.by(() => {
    if (!scanSession.startedAt) {
      return '0s';
    }
    const end = scanSession.completedAt ?? Date.now();
    return `${Math.max(0, Math.round((end - scanSession.startedAt) / 1000))}s`;
  });

  function handleBack() {
    cleanup();
    store.navigate('main');
  }

  function parsePort(): number | null {
    const port = Number.parseInt(portInput.trim(), 10);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      return null;
    }
    return port;
  }

  async function startScan() {
    if (isScanning) {
      return;
    }

    const port = parsePort();
    if (!port) {
      localError = t('scan.portError');
      return;
    }

    localError = null;
    cleanup();
    store.startScanSession(port);

    try {
      unlisten = await listen<ScanProgress>('scan-progress', (progress) => {
        store.updateScanProgress(progress);
      });

      const results = await scanNetwork(port);
      store.devices = await getDevices();
      store.completeScanSession(results);
    } catch (e) {
      store.failScanSession(getErrorMessage(e, t('scan.failed')));
    } finally {
      cleanup();
    }
  }

  function cleanup() {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
  }

  function isAdded(result: ScanResult): boolean {
    return devices.some(
      (d) => d.ip_address === result.ip && d.port === result.port
    );
  }

  async function handleAdd(result: ScanResult) {
    try {
      await addDevice(`Device (${result.ip})`, result.ip, result.port);
      store.devices = await getDevices();
      store.showStatus(t('scan.deviceAdded'));
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('scan.addFailed')));
    }
  }

  $effect(() => {
    if (!startedOnMount) {
      startedOnMount = true;
      startScan();
    }

    return () => {
      cleanup();
    };
  });
</script>

<div class="scan-view">
  <header class="page-header">
    <button class="back-btn" onclick={handleBack} title="Back">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <h1 class="page-title">{t('scan.title')}</h1>
    {#if isScanning}
      <span class="scanning-badge">{t('scan.scanning')}</span>
    {/if}
  </header>

  <section class="scan-controls">
    <label class="port-field">
      <span class="label">{t('scan.adbPort')}</span>
      <input
        class="port-input"
        type="number"
        min="1"
        max="65535"
        bind:value={portInput}
        disabled={isScanning}
      />
    </label>
    <button class="scan-now-btn" onclick={startScan} disabled={isScanning}>
      {isScanning ? t('scan.scanningBtn') : t('scan.scanBtn')}
    </button>
  </section>

  <section class="progress-panel">
    <div class="progress-topline">
      <span class="progress-title">
        {#if isScanning}
          {t('scan.checking')}
        {:else if hasFinished}
          {t('scan.complete')}
        {:else}
          {t('scan.ready')}
        {/if}
      </span>
      <span class="progress-percent">{progressPercent}%</span>
    </div>
    <div class="progress-track" aria-label="Scan progress">
      <div class="progress-fill" style="width: {progressPercent}%;"></div>
    </div>
    <div class="progress-meta">
      <span>{scanSession.progress.scanned}/{scanSession.progress.total} {t('scan.hosts')}</span>
      <span>{visibleResults.length} {t('scan.found')}</span>
      <span>{elapsedLabel}</span>
    </div>
  </section>

  {#if localError || scanSession.error}
    <p class="scan-error">{localError ?? scanSession.error}</p>
  {/if}

  <div class="results-header">
    <span>{t('scan.results')}</span>
    <span>{visibleResults.length}</span>
  </div>

  <div class="results-list">
    {#if visibleResults.length > 0}
      {#each visibleResults as result (`${result.ip}:${result.port}`)}
        <div class="result-row">
          <div class="result-info">
            <span class="result-ip">{result.ip}</span>
            <span class="result-port">:{result.port}</span>
          </div>
          {#if isAdded(result)}
            <span class="added-badge">{t('scan.added')}</span>
          {:else}
            <button class="add-btn" onclick={() => handleAdd(result)}>{t('scan.add')}</button>
          {/if}
        </div>
      {/each}
    {:else}
      <div class="empty-results">
        {#if isScanning}
          <span>{t('scan.waiting')}</span>
        {:else if hasFinished}
          <span>{t('scan.noDevices', { port: scanSession.port })}</span>
        {:else}
          <span>{t('scan.startScan')}</span>
        {/if}
      </div>
    {/if}
  </div>

  <footer class="scan-footer">
    <button class="glass-btn" onclick={startScan} disabled={isScanning}>
      {isScanning ? t('scan.scanningBtn') : t('scan.scanAgain')}
    </button>
    <button class="glass-btn secondary" onclick={handleBack}>
      {t('scan.close')}
    </button>
  </footer>
</div>

<style>
  .scan-view {
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
  }

  .scanning-badge {
    font-size: 10px;
    padding: 3px 8px;
    background: rgba(100, 180, 255, 0.15);
    color: #8cb4ff;
    border-radius: 10px;
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .scan-controls {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    padding: 12px 14px 8px;
    align-items: end;
  }

  .port-field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }

  .label {
    font-size: 10px;
    font-weight: 600;
    color: #777;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .port-input {
    width: 100%;
    min-height: 32px;
    padding: 7px 9px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 13px;
    outline: none;
  }

  .port-input:focus {
    border-color: rgba(100, 180, 255, 0.4);
    box-shadow: 0 0 0 2px rgba(100, 180, 255, 0.1);
  }

  .port-input:disabled {
    opacity: 0.6;
  }

  .scan-now-btn {
    min-width: 72px;
    min-height: 32px;
    padding: 7px 12px;
    background: rgba(100, 180, 255, 0.15);
    border: 1px solid rgba(100, 180, 255, 0.3);
    border-radius: 8px;
    color: #8cb4ff;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  .scan-now-btn:hover {
    background: rgba(100, 180, 255, 0.25);
    border-color: rgba(100, 180, 255, 0.5);
  }

  .scan-now-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .progress-panel {
    margin: 0 12px 8px;
    padding: 10px;
    background: rgba(0, 0, 0, 0.22);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
  }

  .progress-topline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .progress-title {
    font-size: 12px;
    color: #cfcfcf;
  }

  .progress-percent {
    font-size: 12px;
    color: #8cb4ff;
    font-variant-numeric: tabular-nums;
  }

  .progress-track {
    height: 6px;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #8cb4ff, #81c784);
    border-radius: inherit;
    transition: width 0.18s ease;
  }

  .progress-meta {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin-top: 8px;
    color: #888;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .scan-error {
    margin: 0 12px 8px;
    padding: 7px 10px;
    color: #ff8a8a;
    background: rgba(255, 100, 100, 0.08);
    border-radius: 8px;
    font-size: 12px;
  }

  .results-header {
    display: flex;
    justify-content: space-between;
    padding: 6px 14px 4px;
    font-size: 10px;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .results-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px;
  }

  .result-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 38px;
    padding: 8px 10px;
    border-radius: 8px;
    transition: background 0.12s ease;
  }

  .result-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .result-info {
    display: flex;
    align-items: baseline;
    min-width: 0;
  }

  .result-ip {
    font-size: 13px;
    font-weight: 500;
    color: #e0e0e0;
    font-family: 'SF Mono', 'Menlo', monospace;
  }

  .result-port {
    font-size: 12px;
    color: #888;
    font-family: 'SF Mono', 'Menlo', monospace;
  }

  .add-btn {
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 500;
    border: 1px solid rgba(76, 175, 80, 0.3);
    border-radius: 6px;
    background: rgba(76, 175, 80, 0.1);
    color: #81c784;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .add-btn:hover {
    background: rgba(76, 175, 80, 0.2);
    border-color: rgba(76, 175, 80, 0.4);
  }

  .added-badge {
    font-size: 11px;
    padding: 4px 10px;
    background: rgba(255, 255, 255, 0.05);
    color: #777;
    border-radius: 6px;
  }

  .empty-results {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 80px;
    color: #777;
    font-size: 12px;
    text-align: center;
  }

  .scan-footer {
    display: flex;
    gap: 8px;
    padding: 10px 12px 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .glass-btn {
    flex: 1;
    padding: 9px 16px;
    background: rgba(100, 180, 255, 0.15);
    border: 1px solid rgba(100, 180, 255, 0.3);
    border-radius: 8px;
    color: #8cb4ff;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .glass-btn:hover {
    background: rgba(100, 180, 255, 0.25);
    border-color: rgba(100, 180, 255, 0.5);
  }

  .glass-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .glass-btn:active {
    transform: scale(0.97);
  }

  .glass-btn.secondary {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(255, 255, 255, 0.1);
    color: #999;
  }

  .glass-btn.secondary:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #ccc;
  }
</style>
