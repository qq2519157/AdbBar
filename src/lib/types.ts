export interface AdbDevice {
  id: string;
  name: string;
  ip_address: string;
  port: number;
  status: 'disconnected' | 'connecting' | 'connected' | 'unauthorized' | 'offline';
}

export interface ScanResult {
  ip: string;
  port: number;
}

export interface ScanProgress {
  scanned: number;
  total: number;
  found: ScanResult[];
}

export interface ScanSession {
  port: number;
  progress: ScanProgress;
  results: ScanResult[];
  error: string | null;
  startedAt: number | null;
  completedAt: number | null;
}

export interface ScrcpyStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
}
