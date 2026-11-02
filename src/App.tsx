import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Header } from "./components/Header";
import { ButtonGrid } from "./components/ButtonGrid";
import { ButtonEditor } from "./components/ButtonEditor";
import { StatusBar } from "./components/StatusBar";
import { useAppStore } from "./store";
import type { AppConfig } from "./types";

function App() {
  const loadConfig = useAppStore((s) => s.loadConfig);
  const saveConfig = useAppStore((s) => s.saveConfig);
  const setGamingMode = useAppStore((s) => s.setGamingMode);
  const setKeyboardConnected = useAppStore((s) => s.setKeyboardConnected);
  const config = useAppStore((s) => s.config);
  const [selectedButtonId, setSelectedButtonId] = useState<string | null>(null);

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
    const timeout = setTimeout(() => saveConfig(), 500);
    return () => clearTimeout(timeout);
  }, [config, saveConfig]);

  const handleSelectButton = useCallback((buttonId: string) => {
    setSelectedButtonId((prev) => (prev === buttonId ? null : buttonId));
  }, []);

  const handleCloseEditor = useCallback(() => {
    setSelectedButtonId(null);
  }, []);

  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );
  const selectedButton =
    activeProfile?.buttons.find((b) => b.id === selectedButtonId) ?? null;

  return (
    <div className="flex flex-col h-full bg-[var(--color-bg-primary)]">
      <Header />
      <div className="flex flex-1 min-h-0">
        <ButtonGrid
          selectedButtonId={selectedButtonId}
          onSelectButton={handleSelectButton}
        />
        {selectedButton && (
          <ButtonEditor button={selectedButton} onClose={handleCloseEditor} />
        )}
      </div>
      <StatusBar />
    </div>
  );
}

export default App;
