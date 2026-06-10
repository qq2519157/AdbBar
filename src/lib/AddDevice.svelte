<script lang="ts">
  import { store } from './stores.svelte';
  import { addDevice, getDevices } from './api';
  import { getErrorMessage } from './errors';
  import { t } from './i18n';

  let name = $state('');
  let ip = $state('');
  let port = $state('5555');
  let error = $state('');
  let saving = $state(false);

  function handleBack() {
    store.navigate('main');
  }

  function handleCancel() {
    store.navigate('main');
  }

  async function handleAdd() {
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
</div>

<style>
  .add-device {
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

  .form {
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
