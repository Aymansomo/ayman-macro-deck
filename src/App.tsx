import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Header } from "./components/Header";
import { DashboardBar } from "./components/DashboardBar";
import { KeyboardEditor } from "./components/KeyboardEditor";
import { ButtonEditor } from "./components/ButtonEditor";
import { StatusBar } from "./components/StatusBar";
import { SettingsDialog, type SettingsTab } from "./components/SettingsDialog";
import {
  getMacroEngineStatus,
  listenDeckState,
  listenDeviceEvent,
  listenMacroAction,
  listenMacroKey,
  type MacroActionEvent,
  type MacroEngineStatus,
} from "./lib/backend";
import { useAppStore } from "./store";
import type { AppConfig } from "./types";

interface RecentAction {
  id: string;
  buttonId: string;
  buttonName: string;
  keyId?: string;
  message: string;
  ok: boolean;
  at: number;
}

function App() {
  const loadConfig = useAppStore((s) => s.loadConfig);
  const saveConfig = useAppStore((s) => s.saveConfig);
  const setActiveProfile = useAppStore((s) => s.setActiveProfile);
  const setGamingMode = useAppStore((s) => s.setGamingMode);
  const setKeyboardConnected = useAppStore((s) => s.setKeyboardConnected);
  const config = useAppStore((s) => s.config);
  const [selectedButtonId, setSelectedButtonId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState<SettingsTab>("engine");
  const [engineStatus, setEngineStatus] = useState<MacroEngineStatus | null>(null);
  const [activeKeys, setActiveKeys] = useState<Set<string>>(new Set());
  const [runningButtonId, setRunningButtonId] = useState<string | null>(null);
  const [recentActions, setRecentActions] = useState<RecentAction[]>([]);

  const appendRecentAction = useCallback((event: MacroActionEvent, ok: boolean) => {
    const message =
      event.results?.find(Boolean) ?? (ok ? "Completed" : "Action error");
    setRecentActions((prev) =>
      [
        {
          id: crypto.randomUUID(),
          buttonId: event.buttonId,
          buttonName: event.buttonName,
          keyId: event.keyId,
          message,
          ok,
          at: Date.now(),
        },
        ...prev,
      ].slice(0, 6),
    );
  }, []);

  useEffect(() => {
    invoke<string>("load_config")
      .then((json) => {
        if (json && json !== "{}") {
          try {
            const parsed = JSON.parse(json) as AppConfig;
            if (parsed && parsed.version === 1) {
              loadConfig(parsed);
            }
          } catch {
            // malformed JSON — ignore
          }
        }
      })
      .catch(() => {
        const saved = localStorage.getItem("macrodeck-config");
        if (saved) {
          try {
            const parsed = JSON.parse(saved) as AppConfig;
            if (parsed && parsed.version === 1) {
              loadConfig(parsed);
            }
          } catch {
            // ignore
          }
        }
      });

      invoke<boolean>("get_gaming_mode")
        .then((isOn) => setGamingMode(isOn))
        .catch(() => {});

      const modeTimer = setInterval(() => {
        invoke<boolean>("get_gaming_mode")
          .then((isOn) => setGamingMode(isOn))
          .catch(() => {});
      }, 2000);

      invoke<boolean>("get_keyboard_connected")
        .then((connected) => setKeyboardConnected(connected))
        .catch(() => {});

      return () => clearInterval(modeTimer);
    }, [loadConfig, setGamingMode, setKeyboardConnected]);

  useEffect(() => {
    const poll = () => {
      void getMacroEngineStatus().then(setEngineStatus).catch(() => {});
    };
    poll();
    const timer = setInterval(poll, 2000);
    const subscriptions = [
      // This event carries only DeviceStatus. Always re-read the full
      // MacroEngineStatus instead of casting the partial payload, otherwise
      // engineStatus.keyboard becomes undefined and the UI crashes.
      listenDeviceEvent("keyboard-engine-status-changed", () => {
        poll();
      }),
      listenDeviceEvent("keyboard-device-selected", () => poll()),
      listenDeviceEvent("keyboard-device-added", () => poll()),
      listenDeviceEvent("keyboard-device-removed", () => poll()),
    ];
    return () => {
      clearInterval(timer);
      void Promise.all(subscriptions).then((unsubscribers) => {
        unsubscribers.forEach((off) => off());
      });
    };
  }, []);

  useEffect(() => {
    const keySubscriptions = [
      listenMacroKey("macro-key-pressed", ({ keyId }) => {
        setActiveKeys((prev) => new Set(prev).add(keyId));
      }),
      listenMacroKey("macro-key-released", ({ keyId }) => {
        setActiveKeys((prev) => {
          const next = new Set(prev);
          next.delete(keyId);
          return next;
        });
      }),
    ];

    const actionSubscriptions = [
      listenMacroAction("macro-action-started", ({ buttonId }) => {
        setRunningButtonId(buttonId);
      }),
      listenMacroAction("macro-action-completed", (event: MacroActionEvent) => {
        setRunningButtonId(null);
        appendRecentAction(event, true);
      }),
      listenMacroAction("macro-action-error", (event: MacroActionEvent) => {
        setRunningButtonId(null);
        appendRecentAction(event, false);
      }),
    ];

    const all = [...keySubscriptions, ...actionSubscriptions];
    return () => {
      void Promise.all(all).then((unsubscribers) => {
        unsubscribers.forEach((off) => off());
      });
    };
  }, [appendRecentAction]);

  // A profile-switch button pressed on the physical keypad changes the profile
  // inside the backend. Mirror it into the store so the editor, dashboard and
  // config autosave do not keep showing (or writing back) the old profile.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void listenDeckState((state) => {
      const current = useAppStore.getState();
      if (
        state.activeProfileId &&
        state.activeProfileId !== current.config.activeProfileId
      ) {
        setActiveProfile(state.activeProfileId);
      }
      if (state.gamingMode !== current.isGamingMode) {
        setGamingMode(state.gamingMode);
      }
    }).then((off) => {
      if (cancelled) off();
      else unlisten = off;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [setActiveProfile, setGamingMode]);

  useEffect(() => {
    const timeout = setTimeout(() => saveConfig(), 500);
    return () => clearTimeout(timeout);
  }, [config, saveConfig]);

  const handleSelectButton = useCallback((buttonId: string) => {
    setSelectedButtonId((prev) => (prev === buttonId ? null : buttonId));
  }, []);

  const handleCloseEditor = useCallback(() => {
    setSelectedButtonId(null);
  }, []);

  const openSettings = useCallback((tab: SettingsTab) => {
    setSettingsTab(tab);
    setSettingsOpen(true);
  }, []);

  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );
  const selectedButton =
    activeProfile?.buttons.find((b) => b.id === selectedButtonId) ?? null;

  return (
    <div className="app-shell">
      <Header
        engineStatus={engineStatus}
        onOpenSettings={() => openSettings("engine")}
      />
      <div className="app-main">
        <div className="app-workspace">
          <DashboardBar
            engineStatus={engineStatus}
            recentActions={recentActions}
            onOpenDeviceManager={() => openSettings("keypad")}
            onEngineStatusChange={setEngineStatus}
          />
          {activeProfile ? (
            <KeyboardEditor
              profile={activeProfile}
              selectedButtonId={selectedButtonId}
              activeKeys={activeKeys}
              runningButtonId={runningButtonId}
              onSelectButton={handleSelectButton}
            />
          ) : (
            <div className="flex-1 flex items-center justify-center text-[var(--color-text-muted)]">
              No profile selected
            </div>
          )}
        </div>
        {selectedButton && (
          <ButtonEditor
            button={selectedButton}
            onClose={handleCloseEditor}
            running={runningButtonId === selectedButton.id}
            lastAction={recentActions.find(
              (action) => action.buttonId === selectedButton.id,
            )}
          />
        )}
      </div>
      <StatusBar engineStatus={engineStatus} />
      <SettingsDialog
        open={settingsOpen}
        tab={settingsTab}
        onTabChange={setSettingsTab}
        onClose={() => setSettingsOpen(false)}
        selectedButtonId={selectedButtonId}
        onAssignKey={(keyId) => {
          if (!selectedButtonId) return false;
          const ok = useAppStore
            .getState()
            .setButtonPhysicalKey(selectedButtonId, keyId);
          if (ok) void useAppStore.getState().saveConfig();
          return ok;
        }}
      />
    </div>
  );
}

export default App;
