import { useCallback, useEffect, useMemo, useState } from "react";
import {
  clearMacroDeviceSelection,
  confirmDeviceTest,
  getMacroEngineStatus,
  listenDeviceEvent,
  listenDeviceTestResult,
  refreshKeyboardDevices,
  selectMacroDevice,
  setMacroDeviceCollections,
  setMacroDeviceSuppression,
  startDeviceTest,
  stopDeviceTest,
  type DeviceStatus,
  type DeviceTestResult,
  type KeyboardDevice,
} from "../lib/backend";

interface Props {
  selectedButtonId?: string | null;
  onAssignKey?: (keyId: string) => boolean;
}

function shortPath(path: string): string {
  const match = path.match(/VID_[0-9A-F]+&PID_[0-9A-F]+(?:&MI_[0-9A-F]+)?/i);
  return match ? match[0] : path.split("#").slice(0, 2).join("#");
}

function friendlyName(device: KeyboardDevice, index: number, total: number): string {
  const base = device.hardwareId || shortPath(device.path);
  return total > 1 ? `${base} · collection ${index + 1}` : base;
}

export function DeviceManagerContent({ selectedButtonId, onAssignKey }: Props) {
  const [status, setStatus] = useState<DeviceStatus | null>(null);
  const [testing, setTesting] = useState(false);
  const [detected, setDetected] = useState<DeviceTestResult | null>(null);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");

  const refresh = useCallback(async () => {
    try {
      const next = await getMacroEngineStatus();
      setStatus(next.device);
      setTesting(next.device.testing);
    } catch (error) {
      setMessage(String(error));
    }
  }, []);

  const refreshDevices = useCallback(async () => {
    setBusy("refresh");
    setMessage("");
    try {
      await refreshKeyboardDevices();
      await refresh();
      setMessage("Device list refreshed from Windows Raw Input");
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  }, [refresh]);

  useEffect(() => {
    void refresh();

    const subscriptions = [
      listenDeviceEvent("keyboard-device-added", () => void refresh()),
      listenDeviceEvent("keyboard-device-removed", () => void refresh()),
      listenDeviceEvent("keyboard-engine-status-changed", (payload) => {
        const next = payload as DeviceStatus;
        setStatus(next);
        setTesting(next.testing);
      }),
      listenDeviceTestResult((result) => {
        setDetected(result);
        setTesting(false);
      }),
    ];

    return () => {
      void Promise.all(subscriptions).then((unsubscribers) => {
        unsubscribers.forEach((off) => off());
      });
    };
  }, [refresh]);

  const grouped = useMemo(() => {
    const groups = new Map<string, KeyboardDevice[]>();
    for (const device of status?.devices ?? []) {
      const key = device.hardwareId || device.id;
      const list = groups.get(key) ?? [];
      list.push(device);
      groups.set(key, list);
    }
    return [...groups.entries()];
  }, [status?.devices]);

  const handleSelect = async (id: string) => {
    setBusy(id);
    setMessage("");
    try {
      const device = await selectMacroDevice(id);
      setMessage(`Selected ${device.name || device.id}`);
      await refresh();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const handleClear = async () => {
    setBusy("clear");
    try {
      await clearMacroDeviceSelection();
      setMessage("Selection cleared. No keyboard can trigger macros now.");
      await refresh();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const handleStartTest = async () => {
    setBusy("test");
    setDetected(null);
    setMessage("");
    try {
      setTesting(true);
      await startDeviceTest();
      await refresh();
    } catch (error) {
      setTesting(false);
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const handleStopTest = async () => {
    setBusy("test");
    try {
      await stopDeviceTest();
      setTesting(false);
      setDetected(null);
      await refresh();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const handleToggleCollections = async (include: boolean) => {
    setBusy("collections");
    try {
      const next = await setMacroDeviceCollections(include);
      setStatus(next);
      setMessage(
        include
          ? "All HID collections of this keypad can now trigger macros"
          : "Only the exact selected HID collection can trigger macros",
      );
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const handleToggleSuppression = async (suppress: boolean) => {
    setBusy("suppression");
    try {
      const next = await setMacroDeviceSuppression(suppress);
      setStatus(next);
      setMessage(
        suppress
          ? "Keypad captured — deck keys run their macro only, other keyboards still type"
          : "Keypad capture off — deck keys type normally again",
      );
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const handleAssignKey = () => {
    if (!detected) return;
    const ok = onAssignKey?.(detected.keyId) ?? false;
    setMessage(
      ok
        ? `Assigned ${detected.keyId} (${detected.keyLabel}) to the selected key`
        : "Could not assign — select a key first or the key is already used",
    );
  };

  const handleConfirm = async () => {
    if (!detected) return;
    setBusy("confirm");
    try {
      const selected = await confirmDeviceTest();
      setMessage(selected ? `Confirmed ${selected.name || selected.id}` : "No key was detected");
      setTesting(false);
      setDetected(null);
      await refresh();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const statusTone = !status?.selectedId
    ? "bg-[var(--color-danger)]"
    : status.selectedConnected
      ? "bg-[var(--color-success)]"
      : "bg-[var(--color-warning)]";

  const selectedName = status?.devices.find((device) => device.id === status.selectedId)?.name;

  return (
    <div className="space-y-5">
          <section className="rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-card)] p-4 space-y-3">
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-2">
                <span className={`w-2.5 h-2.5 rounded-full ${statusTone}`} />
                <span className="text-xs text-[var(--color-text-secondary)]">
                  {!status?.selectedId
                    ? "No keypad selected"
                    : status.selectedConnected
                      ? "Selected keypad connected"
                      : "Selected keypad disconnected"}
                </span>
              </div>
              <button
                onClick={() => void refreshDevices()}
                disabled={busy === "refresh"}
                className="px-2.5 py-1.5 rounded-lg text-[11px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer disabled:opacity-40"
              >
                ⟳ Refresh
              </button>
            </div>

            <div className="grid grid-cols-2 gap-3 text-[10px]">
              <div>
                <div className="text-[var(--color-text-muted)] tracking-wider mb-1">LISTENER</div>
                <div className="text-[var(--color-text-primary)]">
                  {status?.running ? "Running" : "Stopped"}
                </div>
              </div>
              <div>
                <div className="text-[var(--color-text-muted)] tracking-wider mb-1">
                  DECK KEYS
                </div>
                <div className="text-[var(--color-text-primary)]">
                  {status?.suppressLegacy ? "Macro only" : "Type + macro"}
                </div>
              </div>
            </div>

            <label className="flex items-start gap-2 text-[11px] text-[var(--color-text-secondary)] cursor-pointer">
              <input
                type="checkbox"
                checked={status?.suppressLegacy ?? false}
                onChange={(event) => void handleToggleSuppression(event.target.checked)}
                disabled={busy === "suppression"}
                className="mt-0.5 accent-[var(--color-accent)]"
              />
              <span>
                Keypad keys never type — they only run their macro
                <span className="block text-[10px] text-[var(--color-text-muted)]">
                  Pressing Numpad 1 opens Instagram DMs and no longer types “1”.
                  Windows cannot tell keyboards apart at this layer, so the same
                  key on another numpad is captured too. Pause, or this switch,
                  brings normal typing back.
                </span>
              </span>
            </label>

            <label className="flex items-start gap-2 text-[11px] text-[var(--color-text-secondary)] cursor-pointer">
              <input
                type="checkbox"
                checked={status?.includeCollections ?? false}
                onChange={(event) => void handleToggleCollections(event.target.checked)}
                disabled={busy === "collections"}
                className="mt-0.5 accent-[var(--color-accent)]"
              />
              <span>
                Include all HID collections of this keypad
                <span className="block text-[10px] text-[var(--color-text-muted)]">
                  Turn on if the keypad has several collections and only some
                  keys fire. Off = exact selected collection only.
                </span>
              </span>
            </label>

            <p className="text-[11px] leading-relaxed text-[var(--color-text-secondary)]">
              Bound deck keys are hidden from Windows by the low-level keyboard
              hook, which cannot tell keyboards apart: a key you bind stops typing
              everywhere, and every unbound key types untouched.
            </p>
            {status?.error && (
              <p className="text-[10px] text-[var(--color-danger)]">{status.error}</p>
            )}
          </section>

          <section className="space-y-2">
            <div className="flex items-center justify-between">
              <h3 className="text-[10px] font-medium tracking-wider text-[var(--color-text-muted)]">
                CONNECTED KEYBOARDS ({status?.devices.length ?? 0})
              </h3>
              {status?.selectedId && (
                <button
                  onClick={() => void handleClear()}
                  disabled={busy === "clear"}
                  className="text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-danger)] cursor-pointer disabled:opacity-40"
                >
                  Clear selection
                </button>
              )}
            </div>

            {grouped.length === 0 && (
              <div className="rounded-lg border border-dashed border-[var(--color-border)] px-4 py-8 text-center text-xs text-[var(--color-text-muted)]">
                No keyboard devices reported by Windows.
              </div>
            )}

            <div className="space-y-2">
              {grouped.map(([hardwareId, devices]) => (
                <div
                  key={hardwareId}
                  className="rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-primary)] overflow-hidden"
                >
                  <div className="px-3 py-2 border-b border-[var(--color-border)] flex items-center justify-between">
                    <span className="text-[11px] font-medium text-[var(--color-text-primary)]">
                      {devices[0].hardwareId || shortPath(devices[0].path)}
                    </span>
                    <span className="text-[9px] text-[var(--color-text-muted)]">
                      {devices.length} HID collection{devices.length === 1 ? "" : "s"}
                    </span>
                  </div>
                  {devices.map((device, index) => (
                    <div
                      key={device.id}
                      className={`px-3 py-2.5 flex items-center justify-between gap-3 ${
                        device.selected ? "bg-[var(--color-accent)]/10" : ""
                      }`}
                    >
                      <div className="min-w-0">
                        <div className="text-[11px] text-[var(--color-text-primary)] truncate">
                          {friendlyName(device, index, devices.length)}
                        </div>
                        <div
                          className="text-[9px] text-[var(--color-text-muted)] font-mono truncate max-w-[300px]"
                          title={device.path}
                        >
                          {device.path}
                        </div>
                      </div>
                      <div className="flex items-center gap-2 shrink-0">
                        {device.selected && (
                          <span className="text-[9px] text-[var(--color-accent)]">SELECTED</span>
                        )}
                        <button
                          onClick={() => void handleSelect(device.id)}
                          disabled={busy === device.id}
                          className="px-2.5 py-1 rounded-lg text-[10px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer disabled:opacity-40"
                        >
                          Select
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              ))}
            </div>
          </section>

          <section className="rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-card)] p-4 space-y-3">
            <h3 className="text-[10px] font-medium tracking-wider text-[var(--color-text-muted)]">
              TEST KEYBOARD
            </h3>
            <p className="text-xs leading-relaxed text-[var(--color-text-secondary)]">
              Press any key on the physical keyboard you want to use as the macro
              keypad. The detected device is reported by Windows — no guessing.
            </p>

            {testing ? (
              <div className="rounded-lg border border-[var(--color-accent)]/40 bg-[var(--color-accent)]/10 px-4 py-6 text-center">
                <div className="text-sm text-[var(--color-text-primary)] mb-1">
                  Listening for a key press…
                </div>
                <div className="text-[10px] text-[var(--color-text-muted)]">
                  Press a key on the intended keypad now
                </div>
              </div>
            ) : detected ? (
              <div className="rounded-lg border border-[var(--color-success)]/40 bg-[var(--color-success)]/10 p-3 space-y-2">
                <div className="text-xs text-[var(--color-text-primary)]">
                  Detected: <b>{detected.deviceName}</b>
                </div>
                <div className="text-[11px] font-mono text-[var(--color-text-secondary)]">
                  key {detected.keyId} ({detected.keyLabel})
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={() => void handleConfirm()}
                    disabled={busy === "confirm"}
                    className="flex-1 px-3 py-2 rounded-lg text-xs font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white cursor-pointer disabled:opacity-40"
                  >
                    Use this device
                  </button>
                  <button
                    onClick={() => void handleStopTest()}
                    className="px-3 py-2 rounded-lg text-xs border border-[var(--color-border)] text-[var(--color-text-secondary)] cursor-pointer"
                  >
                    Cancel
                  </button>
                </div>
                {selectedButtonId && onAssignKey && (
                  <button
                    onClick={handleAssignKey}
                    className="w-full px-3 py-2 rounded-lg text-xs border border-[var(--color-accent)]/40 text-[var(--color-accent)] cursor-pointer"
                  >
                    Assign this key to the selected button
                  </button>
                )}
              </div>
            ) : (
              <div className="space-y-2">
                {status?.selectedId && (
                  <div className="rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-primary)] px-3 py-2 text-[11px] text-[var(--color-text-secondary)]">
                    Current keypad:{" "}
                    <span className="text-[var(--color-text-primary)]">
                      {selectedName ?? status.selectedId}
                    </span>
                    {!status.selectedConnected && " (disconnected)"}
                  </div>
                )}
                <button
                  onClick={() => void handleStartTest()}
                  disabled={busy === "test"}
                  className="w-full px-3 py-2 rounded-lg text-xs font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white cursor-pointer disabled:opacity-40"
                >
                  Test Keyboard
                </button>
              </div>
            )}
          </section>

          {message && (
            <p className="text-[11px] text-[var(--color-text-secondary)]">{message}</p>
          )}
    </div>
  );
}
