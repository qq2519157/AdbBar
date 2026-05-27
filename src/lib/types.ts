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

export interface ScrcpyStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
}
