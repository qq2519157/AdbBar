import type { AdbDevice, ScanProgress, ScanResult, ScanSession, ScrcpyStatus } from './types';
import { type Locale, detectLocale } from './i18n';
import { getLocale } from './api';

const initialScanProgress = (found: ScanResult[] = []): ScanProgress => ({
  scanned: 0,
  total: 254,
  found,
});

const createScanSession = (port = 5555): ScanSession => ({
  port,
  progress: initialScanProgress(),
  results: [],
  error: null,
  startedAt: null,
  completedAt: null,
});

class AppStore {
  devices = $state<AdbDevice[]>([]);
  page = $state<'main' | 'addDevice' | 'scan' | 'settings'>('main');
  statusMessage = $state<string | null>(null);
  isRefreshing = $state(false);
  scanSession = $state<ScanSession>(createScanSession());
  isScanning = $state(false);
  adbPath = $state('');
  scrcpyStatus = $state<ScrcpyStatus | null>(null);
  scrcpyInstallLog = $state('');
  isInstallingScrcpy = $state(false);
  locale = $state<Locale>(detectLocale());

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

  startScanSession(port: number) {
    this.scanSession = {
      ...createScanSession(port),
      startedAt: Date.now(),
    };
    this.isScanning = true;
  }

  updateScanProgress(progress: ScanProgress) {
    this.scanSession = {
      ...this.scanSession,
      progress,
    };
  }

  completeScanSession(results: ScanResult[]) {
    this.scanSession = {
      ...this.scanSession,
      results,
      progress: {
        ...this.scanSession.progress,
        scanned: this.scanSession.progress.total,
        found: results,
      },
      error: null,
      completedAt: Date.now(),
    };
    this.isScanning = false;
  }

  failScanSession(error: string) {
    this.scanSession = {
      ...this.scanSession,
      error,
      completedAt: Date.now(),
    };
    this.isScanning = false;
  }

  async initLocale() {
    try {
      const loc = await getLocale();
      if (loc === 'en' || loc === 'zh') {
        this.locale = loc;
      }
    } catch {
      // Use detected locale as fallback
    }
  }
}

export const store = new AppStore();
