import { invoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';
import type { AdbDevice, ScanResult, ScrcpyStatus } from './types';

export async function getDevices(): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>('get_devices');
}

export async function connectDevice(address: string): Promise<string> {
  return invoke<string>('connect_device', { address });
}

export async function disconnectDevice(address: string): Promise<string> {
  return invoke<string>('disconnect_device', { address });
}

export async function refreshAll(): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>('refresh_all');
}

export async function scanNetwork(port: number = 5555): Promise<ScanResult[]> {
  return invoke<ScanResult[]>('scan_network', { port });
}

export async function addDevice(name: string, ipAddress: string, port: number): Promise<void> {
  return invoke<void>('add_device', { name, ipAddress, port });
}

export async function removeDevice(id: string): Promise<void> {
  return invoke<void>('remove_device', { id });
}

export async function openShell(address: string): Promise<void> {
  return invoke<void>('open_shell', { address });
}

export async function launchScrcpy(address: string): Promise<void> {
  return invoke<void>('launch_scrcpy', { address });
}

export async function takeScreenshot(address: string): Promise<string> {
  return invoke<string>('take_screenshot', { address });
}

export async function installApk(address: string, apkPath: string): Promise<string> {
  return invoke<string>('install_apk', { address, apkPath });
}

export async function getAdbPath(): Promise<string> {
  return invoke<string>('get_adb_path');
}

export async function setAdbPath(path: string): Promise<void> {
  return invoke<void>('set_adb_path', { path });
}

export async function detectScrcpyStatus(): Promise<ScrcpyStatus> {
  return invoke<ScrcpyStatus>('detect_scrcpy_status');
}

export async function installScrcpy(): Promise<void> {
  return invoke<void>('install_scrcpy');
}

export async function quitApp(): Promise<void> {
  return invoke<void>('quit_app');
}

export function listen<T>(event: string, handler: (payload: T) => void) {
  return tauriListen<T>(event, (e) => handler(e.payload));
}
