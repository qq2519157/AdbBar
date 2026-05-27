<script lang="ts">
  import { store } from './stores.svelte';
  import { scanNetwork, addDevice, getDevices } from './api';
  import { listen } from './api';
  import type { ScanResult } from './types';

  let scanLogEl: HTMLDivElement | undefined = $state();
  let unlisten: (() => void) | null = $state(null);

  const scanLog = $derived(store.scanLog);
  const scanResults = $derived(store.scanResults);
  const isScanning = $derived(store.isScanning);
  const devices = $derived(store.devices);

  $effect(() => {
    // Auto-scroll log when it updates
    if (scanLogEl) {
      scanLogEl.scrollTop = scanLogEl.scrollHeight;
    }
  });

  function handleBack() {
    cleanup();
    store.navigate('main');
  }

  async function startScan() {
    store.scanLog = '';
    store.scanResults = [];
    store.isScanning = true;

    try {
      // Listen for scan progress events
      unlisten = await listen<{ scanned: number; total: number; found: { ip: string; port: number }[] }>('scan-progress', (progress) => {
        store.scanLog = `Scanning ${progress.scanned}/${progress.total}... Found: ${progress.found.length}\n`;
      });

      const results = await scanNetwork();
      store.scanResults = results;
      store.devices = await getDevices();
      store.scanLog += `\nScan complete. Found ${results.length} device(s).\n`;
    } catch (e) {
      store.scanLog += '\nScan failed.\n';
    } finally {
      store.isScanning = false;
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
      store.showStatus('Device added');
    } catch (e) {
      store.showStatus('Failed to add device');
    }
  }

  $effect(() => {
    startScan();
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
    <h1 class="page-title">Discovered Devices</h1>
    {#if isScanning}
      <span class="scanning-badge">Scanning...</span>
    {/if}
  </header>

  <div class="scan-log" bind:this={scanLogEl}>
    {#if scanLog}
      <pre class="log-text">{scanLog}</pre>
    {:else if isScanning}
      <pre class="log-text">Starting network scan...</pre>
    {:else}
      <pre class="log-text">Ready to scan</pre>
    {/if}
  </div>

  {#if scanResults.length > 0}
    <div class="results-label">Found Devices</div>
    <div class="results-list">
      {#each scanResults as result}
        <div class="result-row">
          <div class="result-info">
            <span class="result-ip">{result.ip}</span>
            <span class="result-port">:{result.port}</span>
          </div>
          {#if isAdded(result)}
            <span class="added-badge">Added</span>
          {:else}
            <button class="add-btn" onclick={() => handleAdd(result)}>Add</button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <footer class="scan-footer">
    <button class="glass-btn" onclick={startScan} disabled={isScanning}>
      {isScanning ? 'Scanning...' : 'Scan Again'}
    </button>
    <button class="glass-btn secondary" onclick={handleBack}>
      Close
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

  .scan-log {
    flex: 0 0 auto;
    max-height: 160px;
    overflow-y: auto;
    margin: 8px 12px;
    padding: 8px 10px;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .log-text {
    margin: 0;
    font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 11px;
    line-height: 1.5;
    color: #8cb4ff;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .results-label {
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
    gap: 0;
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
    color: #666;
    border-radius: 6px;
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
