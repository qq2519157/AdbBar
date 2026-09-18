import { invoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';
import type {
  AdbDevice,
  MdnsService,
  ScanResult,
  ScrcpyStatus,
} from './types';

export async function getDevices(): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>('get_devices');
}

export async function connectDevice(address: string): Promise<string> {
  return invoke<string>('connect_device', { address });
}

export async function pairDevice(address: string, code: string): Promise<string | null> {
  return invoke<string | null>('pair_device', { address, code });
}

export async function disconnectDevice(address: string): Promise<string> {
  return invoke<string>('disconnect_device', { address });
}

export async function disconnectAll(): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>('disconnect_all');
}

export async function refreshAll(reconnect = false): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>('refresh_all', { reconnect });
}

export async function scanNetwork(port: number = 5555): Promise<ScanResult[]> {
  return invoke<ScanResult[]>('scan_network', { port });
}

export async function getMdnsServices(): Promise<MdnsService[]> {
  return invoke<MdnsService[]>('mdns_services');
}

export async function addDevice(name: string, ipAddress: string, port: number): Promise<void> {
  return invoke<void>('add_device', { name, ipAddress, port });
}

export async function removeDevice(id: string): Promise<void> {
  return invoke<void>('remove_device', { id });
}

export async function renameDevice(id: string, name: string): Promise<void> {
  return invoke<void>('rename_device', { id, name });
}

export async function clearDevices(): Promise<void> {
  return invoke<void>('clear_devices');
}

export async function exportDevices(path: string): Promise<void> {
  return invoke<void>('export_devices', { path });
}

export async function importDevices(path: string): Promise<number> {
  return invoke<number>('import_devices', { path });
}

export async function openShell(address: string): Promise<void> {
  return invoke<void>('open_shell', { address });
}

export async function launchScrcpy(address: string): Promise<void> {
  return invoke<void>('launch_scrcpy', { address });
}

export async function setScrcpyOptions(
  bitrateMbps: number | null,
  turnScreenOff: boolean,
  maxSize: number | null,
  stayAwake: boolean
): Promise<void> {
  return invoke<void>('set_scrcpy_options', {
    bitrateMbps,
    turnScreenOff,
    maxSize,
    stayAwake,
  });
}

export async function getScrcpyOptions(): Promise<
  [number | null, boolean, number | null, boolean]
> {
  return invoke<[number | null, boolean, number | null, boolean]>('get_scrcpy_options');
}

export async function getScanPort(): Promise<number | null> {
  return invoke<number | null>('get_scan_port');
}

export async function setScanPort(port: number): Promise<void> {
  return invoke<void>('set_scan_port', { port });
}

export async function setDevicePinned(id: string, pinned: boolean): Promise<void> {
  return invoke<void>('set_device_pinned', { id, pinned });
}

export async function takeScreenshot(address: string): Promise<string> {
  return invoke<string>('take_screenshot', { address });
}

export async function getDeviceProps(address: string): Promise<Record<string, string>> {
  return invoke<Record<string, string>>('get_device_props', { address });
}

export async function installApk(address: string, apkPath: string): Promise<string> {
  return invoke<string>('install_apk', { address, apkPath });
}

export async function getAdbPath(): Promise<string> {
  return invoke<string>('get_adb_path');
}

export async function detectAdbPath(): Promise<string> {
  return invoke<string>('detect_adb_path');
}

export async function setAdbPath(path: string): Promise<void> {
  return invoke<void>('set_adb_path', { path });
}

export async function installAdb(): Promise<string> {
  return invoke<string>('install_adb');
}

export async function checkAdbPath(path: string): Promise<void> {
  return invoke<void>('check_adb_path', { path });
}

export async function detectScrcpyStatus(): Promise<ScrcpyStatus> {
  return invoke<ScrcpyStatus>('detect_scrcpy_status');
}

export async function setScrcpyPath(path: string): Promise<void> {
  return invoke<void>('set_scrcpy_path', { path });
}

export async function installScrcpy(): Promise<void> {
  return invoke<void>('install_scrcpy');
}

export async function quitApp(): Promise<void> {
  return invoke<void>('quit_app');
}

export async function restartAdb(): Promise<string> {
  return invoke<string>('restart_adb');
}

export async function enableTcpip(address?: string, port?: number): Promise<string> {
  return invoke<string>('enable_tcpip', { address, port });
}

export async function getLocale(): Promise<string> {
  return invoke<string>('get_locale');
}

export async function setLocale(locale: string): Promise<void> {
  return invoke<void>('set_locale', { locale });
}

export function listen<T>(event: string, handler: (payload: T) => void) {
  return tauriListen<T>(event, (e) => handler(e.payload));
}
