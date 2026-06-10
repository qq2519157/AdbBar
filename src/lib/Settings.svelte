<script lang="ts">
  import { store } from './stores.svelte';
  import {
    getAdbPath,
    detectAdbPath,
    setAdbPath,
    detectScrcpyStatus,
    setScrcpyPath,
    installScrcpy,
    restartAdb,
    enableTcpip,
    setLocale,
  } from './api';
  import { listen } from './api';
  import { getErrorMessage } from './errors';
  import { t } from './i18n';
  import type { Locale } from './i18n';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getVersion } from '@tauri-apps/api/app';

  let adbPathInput = $state('');
  let appVersion = $state('');
  let adbValid = $state<boolean | null>(null);
  let unlisten: (() => void) | null = $state(null);
  let adbBusy = $state(false);
  let tcpipPort = $state(5555);
  let tcpipBusy = $state(false);
  let scrcpyBusy = $state(false);

  const scrcpyStatus = $derived(store.scrcpyStatus);
  const scrcpyInstallLog = $derived(store.scrcpyInstallLog);
  const isInstallingScrcpy = $derived(store.isInstallingScrcpy);

  let installLogEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (installLogEl) {
      installLogEl.scrollTop = installLogEl.scrollHeight;
    }
  });

  $effect(() => {
    loadSettings();
  });

  $effect(() => {
    getVersion().then((v) => (appVersion = v));
  });

  async function loadSettings() {
    try {
      store.adbPath = await getAdbPath();
      adbPathInput = store.adbPath;
      adbValid = true;
    } catch (e) {
      adbValid = false;
      store.showStatus(getErrorMessage(e, t('settings.adbPath')));
    }
    try {
      store.scrcpyStatus = await detectScrcpyStatus();
    } catch {
      store.scrcpyStatus = { installed: false, path: null, version: null };
    }
  }

  function handleBack() {
    store.navigate('main');
  }

  async function handleSetLocale(loc: Locale) {
    try {
      store.locale = loc;
      await setLocale(loc);
    } catch {
      // ignore
    }
  }

  async function handleBrowse() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
      });
      if (selected) {
        const filePath = typeof selected === 'string' ? selected : selected;
        adbPathInput = filePath as string;
        await applyAdbPath(filePath as string);
      }
    } catch {
      // User cancelled or error
    }
  }

  async function handleAutoDetect() {
    try {
      const path = await detectAdbPath();
      adbPathInput = path;
      if (await applyAdbPath(path)) {
        store.showStatus(t('settings.adbDetected'));
      }
    } catch (e) {
      adbValid = false;
      store.showStatus(getErrorMessage(e, t('settings.adbDetectFailed')));
    }
  }

  async function applyAdbPath(path: string): Promise<boolean> {
    try {
      await setAdbPath(path);
      store.adbPath = path;
      adbValid = true;
      return true;
    } catch (e) {
      adbValid = false;
      store.showStatus(getErrorMessage(e, t('settings.invalidAdbPath')));
      return false;
    }
  }

  async function handleAdbInputBlur() {
    if (adbPathInput.trim()) {
      await applyAdbPath(adbPathInput.trim());
    }
  }

  async function handleRestartAdb() {
    adbBusy = true;
    try {
      const msg = await restartAdb();
      store.showStatus(msg || t('settings.adbRestarted'));
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('settings.restartFailed')));
    } finally {
      adbBusy = false;
    }
  }

  async function handleEnableTcpip() {
    tcpipBusy = true;
    try {
      const msg = await enableTcpip(undefined, tcpipPort);
      store.showStatus(msg || t('settings.tcpipEnabled', { port: tcpipPort }));
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('settings.tcpipFailed')));
    } finally {
      tcpipBusy = false;
    }
  }

  async function handleScrcpyBrowse() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
      });
      if (selected) {
        const filePath = typeof selected === 'string' ? selected : selected;
        scrcpyBusy = true;
        await setScrcpyPath(filePath as string);
        store.scrcpyStatus = await detectScrcpyStatus();
        store.showStatus(t('settings.scrcpyPathSet'));
      }
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('settings.invalidScrcpyPath')));
    } finally {
      scrcpyBusy = false;
    }
  }

  async function handleScrcpyAutoDetect() {
    scrcpyBusy = true;
    try {
      store.scrcpyStatus = await detectScrcpyStatus();
      if (store.scrcpyStatus?.installed) {
        store.showStatus(t('settings.scrcpyDetected'));
      } else {
        store.showStatus(t('settings.scrcpyNotFound'));
      }
    } catch (e) {
      store.showStatus(getErrorMessage(e, t('settings.detectionFailed')));
    } finally {
      scrcpyBusy = false;
    }
  }

  async function handleInstallScrcpy() {
    store.scrcpyInstallLog = '';
    store.isInstallingScrcpy = true;

    try {
      unlisten = await listen<string>('scrcpy-install-progress', (msg) => {
        store.scrcpyInstallLog += msg + '\n';
      });

      await installScrcpy();
      store.scrcpyInstallLog += '\n' + t('settings.installComplete') + '\n';
      store.scrcpyStatus = await detectScrcpyStatus();
      store.showStatus(t('settings.scrcpyInstalled'));
    } catch (e) {
      const message = getErrorMessage(e, t('settings.scrcpyInstallFailed'));
      store.scrcpyInstallLog += `\n${message}\n`;
      store.showStatus(message);
    } finally {
      store.isInstallingScrcpy = false;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    }
  }
</script>

<div class="settings">
  <header class="page-header">
    <button class="back-btn" onclick={handleBack} title="Back">
      <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <h1 class="page-title">{t('settings.title')}</h1>
  </header>

  <div class="settings-content">
    <!-- Language Section -->
    <section class="section">
      <h2 class="section-title">{t('settings.language')}</h2>
      <div class="locale-toggle">
        <button
          class="locale-btn {store.locale === 'en' ? 'active' : ''}"
          onclick={() => handleSetLocale('en')}
        >English</button>
        <button
          class="locale-btn {store.locale === 'zh' ? 'active' : ''}"
          onclick={() => handleSetLocale('zh')}
        >中文</button>
      </div>
    </section>

    <!-- ADB Path Section -->
    <section class="section">
      <h2 class="section-title">{t('settings.adbPath')}</h2>
      <div class="adb-row">
        <div class="input-wrap">
          <input
            class="input"
            type="text"
            placeholder={t('settings.adbPlaceholder')}
            bind:value={adbPathInput}
            onblur={handleAdbInputBlur}
          />
          {#if adbValid === true}
            <span class="validity-indicator valid" title="Valid">&#10003;</span>
          {:else if adbValid === false}
            <span class="validity-indicator invalid" title="Invalid">&#10007;</span>
          {/if}
        </div>
      </div>
      <div class="adb-buttons">
        <button class="glass-btn small" onclick={handleBrowse}>
          <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
          {t('settings.browse')}
        </button>
        <button class="glass-btn small" onclick={handleAutoDetect}>
          <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          {t('settings.autoDetect')}
        </button>
      </div>
    </section>

    <!-- ADB Tools Section -->
    <section class="section">
      <h2 class="section-title">{t('settings.adbTools')}</h2>
      <button
        class="glass-btn full"
        onclick={handleRestartAdb}
        disabled={adbBusy}
      >
        <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 2v6h-6" />
          <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
          <path d="M3 22v-6h6" />
          <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
        </svg>
        {adbBusy ? t('settings.restarting') : t('settings.restartAdb')}
      </button>

      <div class="tcpip-row">
        <div class="input-wrap small">
          <input
            class="input small"
            type="number"
            bind:value={tcpipPort}
            min="1024"
            max="65535"
            disabled={tcpipBusy}
          />
        </div>
        <button
          class="glass-btn"
          onclick={handleEnableTcpip}
          disabled={tcpipBusy}
        >
          <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M5 12.55a11 11 0 0 1 14.08 0" />
            <path d="M1.42 9a16 16 0 0 1 21.16 0" />
            <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
            <circle cx="12" cy="20" r="1" fill="currentColor" />
          </svg>
          {tcpipBusy ? t('settings.enabling') : t('settings.enableTcpip')}
        </button>
      </div>
      <p class="hint">{t('settings.tcpipHint')}</p>
    </section>

    <!-- Scrcpy Section -->
    <section class="section">
      <h2 class="section-title">{t('settings.scrcpy')}</h2>
      <div class="scrcpy-status">
        {#if scrcpyStatus}
          {#if scrcpyStatus.installed}
            <div class="status-row">
              <span class="status-dot installed"></span>
              <span class="status-text">{t('settings.installed')}</span>
              {#if scrcpyStatus.version}
                <span class="status-version">v{scrcpyStatus.version}</span>
              {/if}
            </div>
            {#if scrcpyStatus.path}
              <div class="status-path">{scrcpyStatus.path}</div>
            {/if}
          {:else}
            <div class="status-row">
              <span class="status-dot not-installed"></span>
              <span class="status-text">{t('settings.notInstalled')}</span>
            </div>
          {/if}
        {:else}
          <div class="status-row">
            <span class="status-text">{t('settings.checking')}</span>
          </div>
        {/if}
      </div>

      <div class="adb-buttons">
        <button class="glass-btn small" onclick={handleScrcpyBrowse} disabled={scrcpyBusy}>
          <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
          {t('settings.browse')}
        </button>
        <button class="glass-btn small" onclick={handleScrcpyAutoDetect} disabled={scrcpyBusy}>
          <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          {t('settings.autoDetect')}
        </button>
        <button
          class="glass-btn small"
          onclick={handleInstallScrcpy}
          disabled={isInstallingScrcpy}
        >
          {#if isInstallingScrcpy}
            {t('settings.installing')}
          {:else if scrcpyStatus?.installed}
            {t('settings.reinstall')}
          {:else}
            {t('settings.install')}
          {/if}
        </button>
      </div>

      {#if scrcpyInstallLog}
        <div class="install-log" bind:this={installLogEl}>
          <pre class="log-text">{scrcpyInstallLog}</pre>
        </div>
      {/if}
    </section>

    <!-- About Section -->
    <section class="section">
      <h2 class="section-title">{t('settings.about')}</h2>
      <div class="about-info">
        <div class="about-row">
          <span class="about-label">ADB Bar</span>
          {#if appVersion}
            <span class="about-version">v{appVersion}</span>
          {:else}
            <span class="about-version">{t('settings.loading')}</span>
          {/if}
        </div>
      </div>
    </section>
  </div>
</div>

<style>
  .settings {
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
  }

  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px 14px;
  }

  .section {
    margin-bottom: 20px;
  }

  .locale-toggle {
    display: flex;
    gap: 6px;
  }

  .locale-btn {
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

  .locale-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #ccc;
  }

  .locale-btn.active {
    background: rgba(100, 180, 255, 0.15);
    border-color: rgba(100, 180, 255, 0.3);
    color: #8cb4ff;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0 0 10px;
  }

  .adb-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .input-wrap {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }

  .input {
    width: 100%;
    padding: 8px 10px;
    padding-right: 28px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 12px;
    font-family: 'SF Mono', 'Menlo', monospace;
    outline: none;
    transition: all 0.15s ease;
  }

  .input::placeholder {
    color: #555;
  }

  .input:focus {
    border-color: rgba(100, 180, 255, 0.4);
    background: rgba(255, 255, 255, 0.08);
  }

  .validity-indicator {
    position: absolute;
    right: 8px;
    font-size: 14px;
    font-weight: bold;
  }

  .validity-indicator.valid {
    color: #4caf50;
  }

  .validity-indicator.invalid {
    color: #f44336;
  }

  .adb-buttons {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .glass-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 14px;
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

  .glass-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .glass-btn.small {
    padding: 6px 12px;
    font-size: 11px;
  }

  .glass-btn.full {
    width: 100%;
    margin-top: 8px;
  }

  .tcpip-row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
    align-items: center;
  }

  .input-wrap.small {
    width: 72px;
    flex: none;
  }

  .input.small {
    width: 100%;
    padding: 6px 8px;
    padding-right: 8px;
    text-align: center;
    font-size: 12px;
  }

  .tcpip-row .glass-btn {
    flex: 1;
  }

  .hint {
    margin: 6px 0 0;
    font-size: 10px;
    color: #666;
  }

  .btn-icon {
    width: 13px;
    height: 13px;
  }

  .scrcpy-status {
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-dot.installed {
    background: #4caf50;
    box-shadow: 0 0 4px #4caf50;
  }

  .status-dot.not-installed {
    background: #666;
  }

  .status-text {
    font-size: 12px;
    color: #ccc;
  }

  .status-version {
    font-size: 11px;
    color: #888;
    font-family: 'SF Mono', 'Menlo', monospace;
  }

  .status-path {
    margin-top: 6px;
    font-size: 11px;
    color: #666;
    font-family: 'SF Mono', 'Menlo', monospace;
    word-break: break-all;
    padding-left: 16px;
  }

  .install-log {
    margin-top: 10px;
    max-height: 120px;
    overflow-y: auto;
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

  .about-info {
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .about-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .about-label {
    font-size: 12px;
    color: #ccc;
    font-weight: 500;
  }

  .about-version {
    font-size: 11px;
    color: #888;
    font-family: 'SF Mono', 'Menlo', monospace;
  }
</style>
