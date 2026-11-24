import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type KeyboardLayout = "full" | "numpad" | "numpadNav";

export interface MonitorInfo {
  index: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  primary: boolean;
}

export function listMonitors(): Promise<MonitorInfo[]> {
  return invoke<MonitorInfo[]>("list_monitors");
}

export interface RunningApp {
  pid: number;
  title: string;
  path: string;
  name: string;
}

/** Apps that currently have a window (tray apps included). */
export function listRunningApps(): Promise<RunningApp[]> {
  return invoke<RunningApp[]>("list_running_apps");
}

/** Runs a profile's launch-on-activate apps right now, in the background. */
export function applyProfileApps(profileId: string): Promise<void> {
  return invoke<void>("apply_profile_apps", { profileId });
}

export interface ProfileAppResult {
  profileId: string;
  line: string;
  done: boolean;
}

/** One finished app of a run, in order; `done` marks the last one. */
export function listenProfileAppsResult(
  handler: (event: ProfileAppResult) => void,
) {
  return listen<ProfileAppResult>("profile-apps-result", ({ payload }) =>
    handler(payload),
  );
}

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

export interface PowerActionEvent {
  /** "shutdown_pc" or "restart_pc". */
  actionType: string;
  label: string;
  /** Only on the armed event: how long the confirmation stays open. */
  windowSecs?: number;
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

export interface BackupSummary {
  path: string;
  profiles: number;
  buttons: number;
}

/** Writes every profile, page, button, action and setting (plus the open page)
 *  into one JSON file the user picks. */
export function backupData(destination: string) {
  return invoke<BackupSummary>("backup_data", { destination });
}

/** Restores a file written by `backupData`; the current config is backed up first. */
export function restoreData(source: string) {
  return invoke<BackupSummary>("restore_data", { source });
}

/** Opens %APPDATA%\com.ayman.macrodeck in Explorer. */
export function openDataFolder() {
  return invoke<void>("open_data_folder");
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

/** Shut Down / Restart was pressed once and is waiting for a confirmation. */
export function listenPowerActionArmed(handler: (event: PowerActionEvent) => void) {
  return listen<PowerActionEvent>("power-action-armed", ({ payload }) => handler(payload));
}

/** Cancels a pending Shut Down / Restart confirmation in the engine. */
export function cancelPowerAction(actionType: string) {
  return invoke<boolean>("cancel_power_action", { actionType });
}

/** The confirmation press ran the action, so its banner can go away. */
export function listenPowerActionConfirmed(handler: (event: PowerActionEvent) => void) {
  return listen<PowerActionEvent>("power-action-confirmed", ({ payload }) =>
    handler(payload),
  );
}

export function listenMacroKey(
  event: "macro-key-pressed" | "macro-key-released",
  handler: (result: MacroKeyEvent) => void,
) {
  return listen<MacroKeyEvent>(event, ({ payload }) => handler(payload));
}
