<script lang="ts">
  import { store } from './lib/stores';
  import DeviceList from './lib/DeviceList.svelte';
  import AddDevice from './lib/AddDevice.svelte';
  import ScanView from './lib/ScanView.svelte';
  import Settings from './lib/Settings.svelte';

  const page = $derived(store.page);
</script>

<div class="app-container">
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

<style>
  .app-container {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: rgba(30, 30, 30, 0.85);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border-radius: 12px;
    overflow: hidden;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    font-size: 13px;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
