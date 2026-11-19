import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type KeyboardLayout = "full" | "numpad" | "numpadNav";

export interface KeyboardDevice {
  id: string;
  name: string;
  path: string;
  hardwareId: string;
  connected: boolean;
  selected: boolean;
}

export interface DeviceStatus {
  running: boolean;
  error: string | null;
  devices: KeyboardDevice[];
  selectedId: string | null;
  selectedConnected: boolean;
  testing: boolean;
  suppressLegacy: boolean;
  includeCollections: boolean;
}

export interface DeviceTestResult {
  deviceId: string;
  deviceName: string;
  keyId: string;
  keyLabel: string;
}

export interface ShortcutInfo {
  shortcut: string;
  buttonId: string;
  buttonName: string;
}

export interface KeyboardStatus {
  running: boolean;
  paused: boolean;
  error: string | null;
  shortcutCount: number;
  shortcuts: ShortcutInfo[];
  eventsSeen: number;
  triggers: number;
}

export interface MacroEngineStatus {
  device: DeviceStatus;
  keyboard: KeyboardStatus;
}

export interface MacroActionEvent {
  deviceId?: string;
  keyId?: string;
  keyLabel?: string;
  buttonId: string;
  buttonName: string;
  results?: string[];
  gamingMode?: boolean;
  shortcut?: string;
}

export interface MacroKeyEvent {
  deviceId?: string;
  keyId: string;
  keyLabel?: string;
}

export function getKeyboardDevices() {
  return invoke<KeyboardDevice[]>("get_keyboard_devices");
}

export function refreshKeyboardDevices() {
  return invoke<KeyboardDevice[]>("refresh_keyboard_devices");
}

export function getSelectedMacroDevice() {
  return invoke<KeyboardDevice | null>("get_selected_macro_device");
}

export function selectMacroDevice(id: string) {
  return invoke<KeyboardDevice>("select_macro_device", { deviceId: id });
}

export function clearMacroDeviceSelection() {
  return invoke<void>("clear_macro_device_selection");
}

export function startDeviceTest() {
  return invoke<DeviceStatus>("start_device_test");
}

export function stopDeviceTest() {
  return invoke<DeviceStatus>("stop_device_test");
}

export function getDeviceTestCandidate() {
  return invoke<KeyboardDevice | null>("get_device_test_candidate");
}

export function confirmDeviceTest() {
  return invoke<KeyboardDevice | null>("confirm_device_test");
}

export function getMacroEngineStatus() {
  return invoke<MacroEngineStatus>("get_macro_engine_status");
}

export function setMacroEngineRunning(running: boolean) {
  return invoke<MacroEngineStatus>("set_macro_engine_running", { running });
}

export function setMacroDeviceCollections(include: boolean) {
  return invoke<DeviceStatus>("set_macro_device_collections", { include });
}

export function setMacroDeviceSuppression(suppress: boolean) {
  return invoke<DeviceStatus>("set_macro_device_suppression", { suppress });
}

export function getStartWithWindows() {
  return invoke<boolean>("get_start_with_windows");
}

export function setStartWithWindows(enabled: boolean) {
  return invoke<boolean>("set_start_with_windows", { enabled });
}

export function loadConfigRaw() {
  return invoke<string>("load_config");
}

export function openLogsFolder() {
  return invoke<void>("open_logs_folder");
}

export function exportConfig() {
  return invoke<string>("export_config");
}

export function importConfig(config: string) {
  return invoke<void>("import_config", { config });
}

export function listConfigBackups() {
  return invoke<string[]>("list_config_backups");
}

export function restoreConfigBackup(name: string) {
  return invoke<void>("restore_config_backup", { name });
}

export interface ApplicationInfo {
  path: string;
  name: string;
  iconPath: string | null;
}

export function getApplicationInfo(path: string) {
  return invoke<ApplicationInfo>("get_application_info", { path });
}

export function listenDeviceEvent(
  event:
    | "keyboard-device-added"
    | "keyboard-device-removed"
    | "keyboard-device-selected"
    | "keyboard-engine-status-changed",
  handler: (payload: unknown) => void,
) {
  return listen(event, ({ payload }) => handler(payload));
}

export function listenDeviceTestResult(handler: (result: DeviceTestResult) => void) {
  return listen<DeviceTestResult>("device-test-result", ({ payload }) => handler(payload));
}

export function listenMacroAction(
  event: "macro-action-started" | "macro-action-completed" | "macro-action-error",
  handler: (result: MacroActionEvent) => void,
) {
  return listen<MacroActionEvent>(event, ({ payload }) => handler(payload));
}

export interface DeckState {
  activeProfileId: string;
  activeProfileName: string | null;
  boundProfileName: string | null;
  gamingMode: boolean;
  mode: string;
}

export function listenDeckState(handler: (state: DeckState) => void) {
  return listen<DeckState>("deck:state", ({ payload }) => handler(payload));
}

export function listenMacroKey(
  event: "macro-key-pressed" | "macro-key-released",
  handler: (result: MacroKeyEvent) => void,
) {
  return listen<MacroKeyEvent>(event, ({ payload }) => handler(payload));
}
