<script lang="ts">
  import { store } from './stores.svelte';
  import {
    addDevice,
    getDevices,
    pairDevice,
    getMdnsServices,
    connectDevice,
    refreshAll,
  } from './api';
  import { getErrorMessage } from './errors';
  import { t } from './i18n';
  import type { MdnsService } from './types';

  let mode = $state<'manual' | 'pair'>('manual');
  let name = $state('');
  let ip = $state('');
  let port = $state('5555');
  let pairAddress = $state('');
  let pairCode = $state('');
  let error = $state('');
  let saving = $state(false);
  let pairing = $state(false);

  // Consume a prefill handed over from mDNS discovery (scan page).
  {
    const prefill = store.addDevicePrefill;
    if (prefill) {
      store.addDevicePrefill = null;
      mode = prefill.mode;
      if (prefill.pairAddress !== undefined) pairAddress = prefill.pairAddress;
      if (prefill.ip !== undefined) ip = prefill.ip;
      if (prefill.port !== undefined) port = prefill.port;
    }
  }

  function switchMode(next: 'manual' | 'pair') {
    error = '';
    mode = next;
    if (next === 'pair') {
      loadDiscovered();
    }
  }

  function handleBack() {
    store.navigate('main');
  }

  function handleCancel() {
    store.navigate('main');
  }

  async function handleAdd() {
    if (saving) {
      return;
    }
    error = '';

    if (!name.trim()) {
      error = t('addDevice.nameRequired');
      return;
    }
    if (!ip.trim()) {
      error = t('addDevice.ipRequired');
      return;
    }

    const portNum = parseInt(port, 10);
    if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
      error = t('addDevice.portError');
      return;
    }

    saving = true;
    try {
      await addDevice(name.trim(), ip.trim(), portNum);
      store.devices = await getDevices();
      store.showStatus(t('addDevice.deviceAdded'));
      store.navigate('main');
    } catch (e) {
      error = getErrorMessage(e, t('addDevice.addFailed'));
    } finally {
      saving = false;
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    handleAdd();
  }

  async function handlePair() {
    if (pairing) {
      return;
    }
    const addr = pairAddress.trim();
    const code = pairCode.trim();
    if (!addr) {
      error = t('addDevice.pairAddrRequired');
      return;
    }
    if (!code) {
      error = t('addDevice.pairCodeRequired');
      return;
    }
    error = '';
    pairing = true;
    try {
      const connectAddress = await pairDevice(addr, code);
      if (connectAddress) {
        await addFromConnectAddress(connectAddress);
      } else {
        // No connect service discovered: fall back to the manual flow with the
        // host prefilled; the connect port must be read off the phone.
        ip = addr.split(':')[0] || '';
        name = '';
        port = '5555';
        switchMode('manual');
        store.showStatus(t('addDevice.paired'));
      }
    } catch (e) {
      error = getErrorMessage(e, t('addDevice.pairFailed'));
    } finally {
      pairing = false;
    }
  }

  // --- mDNS discovery (Android Studio "Select device" style) ---
  // Connectable services are tried directly — pairing is only for devices that
  // were never paired with this Mac (the pairing service shows up while the
  // phone's pairing-code dialog is open).

  let discovered = $state<MdnsService[] | null>(null);
  let discoveredLoading = $state(false);
  let connectingAddress = $state('');
  let codeInputEl: HTMLInputElement | undefined = $state();

  const pairableText = $derived(
    discoveredLoading ? t('deviceDetail.loading') : t('addDevice.discoveredEmpty')
  );

  async function loadDiscovered() {
    discoveredLoading = true;
    try {
      discovered = await getMdnsServices();
    } catch {
      discovered = null;
    } finally {
      discoveredLoading = false;
    }
  }

  function pickPairable(svc: MdnsService) {
    pairAddress = svc.address;
    error = '';
    codeInputEl?.focus();
    codeInputEl?.select();
  }

  async function connectDiscovered(svc: MdnsService) {
    if (connectingAddress) {
      return;
    }
    connectingAddress = svc.address;
    error = '';
    try {
      const idx = svc.address.lastIndexOf(':');
      const host = idx === -1 ? svc.address : svc.address.slice(0, idx);
      const portNum =
        idx === -1 ? 5555 : Number.parseInt(svc.address.slice(idx + 1), 10) || 5555;
      try {
        await addDevice(`Device (${host})`, host, portNum);
      } catch {
        // Already saved — fall through and just connect.
      }
      await connectDevice(svc.address);
      store.devices = await refreshAll();
      store.showStatus(t('addDevice.connectedDirect'));
      store.navigate('main');
    } catch (e) {
      // Most likely never paired with this Mac: guide to the pairing flow.
      error = t('addDevice.connectNeedsPair');
      const detail = getErrorMessage(e, '');
      if (detail) {
        error += `\n${detail}`;
      }
      await loadDiscovered();
    } finally {
      connectingAddress = '';
    }
  }

  async function addFromConnectAddress(connectAddress: string) {    const idx = connectAddress.lastIndexOf(':');
    const host = idx === -1 ? connectAddress : connectAddress.slice(0, idx);
    const portNum = idx === -1 ? 5555 : Number.parseInt(connectAddress.slice(idx + 1), 10) || 5555;
    try {
      await addDevice(`Device (${host})`, host, portNum);
      store.devices = await getDevices();
      store.showStatus(t('addDevice.pairedAdded'));
      store.navigate('main');
    } catch {
      // Already saved — prefill the manual form instead.
      name = '';
      ip = host;
      port = String(portNum);
      switchMode('manual');
      store.showStatus(t('addDevice.paired'));
    }
  }

  function handlePairSubmit(e: Event) {
    e.preventDefault();
    handlePair();
  }

  // After all declarations: load discovered devices when landing in pair mode.
  if (mode === 'pair') {
    loadDiscovered();
  }
</script>

<div class="add-device">
  <header class="page-header">
    <button class="back-btn" onclick={handleBack} title="Back">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <h1 class="page-title">{t('addDevice.title')}</h1>
  </header>

  <div class="mode-toggle">
    <button class="mode-btn {mode === 'manual' ? 'active' : ''}" onclick={() => switchMode('manual')}>
      {t('addDevice.modeManual')}
    </button>
    <button class="mode-btn {mode === 'pair' ? 'active' : ''}" onclick={() => switchMode('pair')}>
      {t('addDevice.modePair')}
    </button>
  </div>

  {#if mode === 'pair'}
    <form class="form" onsubmit={handlePairSubmit}>
      <div class="pairable-panel">
        <div class="pairable-header">
          <span class="pairable-title">{t('addDevice.discoveredDevices')}</span>
          <button
            type="button"
            class="pairable-refresh"
            onclick={loadDiscovered}
            disabled={discoveredLoading}
            title={t('scan.mdnsRefresh')}
          >
            <svg class="icon {discoveredLoading ? 'spinning' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 2v6h-6" />
              <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
              <path d="M3 22v-6h6" />
              <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
            </svg>
          </button>
        </div>
        {#if discovered && discovered.length > 0}
          {#each discovered as svc (`${svc.address}|${svc.kind}`)}
            <div class="pairable-row" class:selected={pairAddress === svc.address}>
              <button type="button" class="pairable-info" onclick={() => svc.kind === 'pairing' && pickPairable(svc)}>
                <span class="pairable-name">{svc.name}</span>
                <span class="pairable-addr">{svc.address}</span>
              </button>
              <span class="mdns-badge {svc.kind}">
                {svc.kind === 'pairing' ? t('scan.mdnsPairable') : t('scan.mdnsConnectable')}
              </span>
              {#if svc.kind === 'pairing'}
                <button type="button" class="pair-action" onclick={() => pickPairable(svc)}>
                  {t('scan.mdnsPairAction')}
                </button>
              {:else}
                <button
                  type="button"
                  class="pair-action connect"
                  onclick={() => connectDiscovered(svc)}
                  disabled={connectingAddress !== ''}
                >
                  {connectingAddress === svc.address ? t('deviceRow.connecting') : t('addDevice.connectAction')}
                </button>
              {/if}
            </div>
          {/each}
        {:else}
          <p class="pair-hint">
            {pairableText}
          </p>
        {/if}
      </div>

      <label class="field">
        <span class="label">{t('addDevice.pairAddr')}</span>
        <input
          class="input"
          type="text"
          placeholder={t('addDevice.pairAddrPlaceholder')}
          bind:value={pairAddress}
          autocomplete="off"
        />
      </label>

      <label class="field">
        <span class="label">{t('addDevice.pairCode')}</span>
        <input
          class="input"
          type="text"
          inputmode="numeric"
          maxlength="12"
          placeholder={t('addDevice.pairCodePlaceholder')}
          bind:value={pairCode}
          bind:this={codeInputEl}
          autocomplete="off"
        />
      </label>

      <p class="pair-hint">{t('addDevice.pairHint')}</p>

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="actions">
        <button type="button" class="glass-btn secondary" onclick={handleCancel}>{t('addDevice.cancel')}</button>
        <button type="submit" class="glass-btn primary" disabled={pairing}>
          {pairing ? t('addDevice.pairing') : t('addDevice.pairBtn')}
        </button>
      </div>
    </form>
  {:else}
    <form class="form" onsubmit={handleSubmit}>
    <label class="field">
      <span class="label">{t('addDevice.deviceName')}</span>
      <input
        class="input"
        type="text"
        placeholder={t('addDevice.namePlaceholder')}
        bind:value={name}
        autocomplete="off"
      />
    </label>

    <label class="field">
      <span class="label">{t('addDevice.ipAddress')}</span>
      <input
        class="input"
        type="text"
        placeholder={t('addDevice.ipPlaceholder')}
        bind:value={ip}
        autocomplete="off"
      />
    </label>

    <label class="field">
      <span class="label">{t('addDevice.port')}</span>
      <input
        class="input"
        type="number"
        placeholder={t('addDevice.portPlaceholder')}
        bind:value={port}
        min="1"
        max="65535"
      />
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}

      <div class="actions">
        <button type="button" class="glass-btn secondary" onclick={handleCancel}>{t('addDevice.cancel')}</button>
        <button type="submit" class="glass-btn primary" disabled={saving}>
          {saving ? t('addDevice.adding') : t('addDevice.addBtn')}
        </button>
      </div>
    </form>
  {/if}
</div>

<style>
  .add-device {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .mode-toggle {
    display: flex;
    gap: 6px;
    padding: 12px 14px 0;
  }

  .mode-btn {
    flex: 1;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #999;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #ccc;
  }

  .mode-btn.active {
    background: rgba(100, 180, 255, 0.15);
    border-color: rgba(100, 180, 255, 0.3);
    color: #8cb4ff;
  }

  .pair-hint {
    margin: 0;
    font-size: 10px;
    color: #666;
    line-height: 1.5;
  }

  .pairable-panel {
    padding: 10px 12px;
    background: rgba(0, 0, 0, 0.22);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
  }

  .pairable-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .pairable-title {
    font-size: 10px;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .pairable-refresh {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    color: #b0b0b0;
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pairable-refresh:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
  }

  .pairable-refresh:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .pairable-refresh .icon {
    width: 13px;
    height: 13px;
  }

  .spinning {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .pairable-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    margin-bottom: 4px;
    border: 1px solid transparent;
    border-radius: 6px;
    transition: all 0.12s ease;
  }

  .pairable-row:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .pairable-row.selected {
    background: rgba(100, 180, 255, 0.12);
    border-color: rgba(100, 180, 255, 0.35);
  }

  .pairable-info {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }

  .pairable-name {
    font-size: 12px;
    color: #e8e8e8;
    font-family: 'SF Mono', 'Menlo', monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pairable-addr {
    font-size: 11px;
    color: #888;
    font-family: 'SF Mono', 'Menlo', monospace;
    flex-shrink: 0;
  }

  .mdns-badge {
    font-size: 10px;
    padding: 3px 8px;
    border-radius: 10px;
    flex-shrink: 0;
  }

  .mdns-badge.pairing {
    background: rgba(255, 152, 0, 0.15);
    color: #ffb74d;
  }

  .mdns-badge.connect {
    background: rgba(76, 175, 80, 0.15);
    color: #81c784;
  }

  .pair-action {
    flex-shrink: 0;
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 500;
    border: 1px solid rgba(255, 152, 0, 0.35);
    border-radius: 6px;
    background: rgba(255, 152, 0, 0.12);
    color: #ffb74d;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .pair-action:hover {
    background: rgba(255, 152, 0, 0.22);
  }

  .pair-action.connect {
    border-color: rgba(76, 175, 80, 0.35);
    background: rgba(76, 175, 80, 0.12);
    color: #81c784;
  }

  .pair-action.connect:hover {
    background: rgba(76, 175, 80, 0.22);
  }

  .pair-action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
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
  }

  .form {
    flex: 1;
    overflow-y: auto;
    padding: 16px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .label {
    font-size: 11px;
    font-weight: 500;
    color: #999;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .input {
    padding: 8px 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 13px;
    outline: none;
    transition: all 0.15s ease;
    font-family: inherit;
  }

  .input::placeholder {
    color: #555;
  }

  .input:focus {
    border-color: rgba(100, 180, 255, 0.4);
    background: rgba(255, 255, 255, 0.08);
    box-shadow: 0 0 0 2px rgba(100, 180, 255, 0.1);
  }

  .error {
    color: #ff8a8a;
    font-size: 12px;
    margin: 0;
    padding: 6px 10px;
    background: rgba(255, 100, 100, 0.08);
    border-radius: 6px;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .glass-btn {
    flex: 1;
    padding: 9px 16px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid;
  }

  .glass-btn.primary {
    background: rgba(100, 180, 255, 0.15);
    border-color: rgba(100, 180, 255, 0.3);
    color: #8cb4ff;
  }

  .glass-btn.primary:hover {
    background: rgba(100, 180, 255, 0.25);
    border-color: rgba(100, 180, 255, 0.5);
  }

  .glass-btn.primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
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

  .glass-btn:active {
    transform: scale(0.97);
  }
</style>
