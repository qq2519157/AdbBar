import type { AdbDevice, ScanResult, ScrcpyStatus } from './types';

class AppStore {
  devices = $state<AdbDevice[]>([]);
  page = $state<'main' | 'addDevice' | 'scan' | 'settings'>('main');
  statusMessage = $state<string | null>(null);
  isRefreshing = $state(false);
  scanLog = $state('');
  scanResults = $state<ScanResult[]>([]);
  isScanning = $state(false);
  adbPath = $state('');
  scrcpyStatus = $state<ScrcpyStatus | null>(null);
  scrcpyInstallLog = $state('');
  isInstallingScrcpy = $state(false);

  showStatus(msg: string, duration = 3000) {
    this.statusMessage = msg;
    setTimeout(() => {
      if (this.statusMessage === msg) {
        this.statusMessage = null;
      }
    }, duration);
  }

  navigate(page: 'main' | 'addDevice' | 'scan' | 'settings') {
    this.page = page;
  }
}

export const store = new AppStore();
