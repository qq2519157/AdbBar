import { invoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';
import type { AdbDevice, ScanResult, ScrcpyStatus } from './types';

export async function getDevices(): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>('get_devices');
}

export async function connectDevice(address: string): Promise<{ success: boolean; message: string }> {
  return invoke<{ success: boolean; message: string }>('connect_device', { address });
}

export async function disconnectDevice(address: string): Promise<{ success: boolean; message: string }> {
  return invoke<{ success: boolean; message: string }>('disconnect_device', { address });
}

export async function refreshAll(): Promise<AdbDevice[]> {
  return invoke<AdbDevice[]>('refresh_all');
}

export async function scanNetwork(): Promise<ScanResult[]> {
  return invoke<ScanResult[]>('scan_network');
}

export async function addDevice(name: string, ip: string, port: number): Promise<void> {
  return invoke<void>('add_device', { name, ip, port });
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

export async function takeScreenshot(address: string): Promise<{ success: boolean; message: string }> {
  return invoke<{ success: boolean; message: string }>('take_screenshot', { address });
}

export async function installApk(address: string, path: string): Promise<{ success: boolean; message: string }> {
  return invoke<{ success: boolean; message: string }>('install_apk', { address, path });
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

export function listen<T>(event: string, handler: (payload: T) => void) {
  return tauriListen<T>(event, (e) => handler(e.payload));
}
