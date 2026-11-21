import { useCallback, useEffect, useRef, useState } from "react";
import {
  type Connection,
  type DeckState,
  clearConnection,
  fetchState,
  loadConnection,
  pair,
  parseHostInput,
  ping,
  pressButton,
  saveConnection,
} from "./api";

type Screen = "setup" | "deck";

function keyLabel(key: number): string {
  if (key === 10) return "/";
  if (key === 11) return "*";
  return String(key);
}

function RefreshIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M20 11a8 8 0 1 0-2.6 5.9M20 4v7h-7"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="3" stroke="currentColor" strokeWidth="1.8" />
      <path
        d="M12 3.5v2M12 18.5v2M3.5 12h2M18.5 12h2M6 6l1.5 1.5M16.5 16.5 18 18M18 6l-1.5 1.5M7.5 16.5 6 18"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
    </svg>
  );
}

function WifiIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M4 9.5a12 12 0 0 1 16 0M7 13a8 8 0 0 1 10 0M10 16.4a3.6 3.6 0 0 1 4 0"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
      <circle cx="12" cy="19" r="1.3" fill="currentColor" />
    </svg>
  );
}

function Spinner() {
  return <span className="spinner" aria-hidden="true" />;
}

export default function App() {
  const [screen, setScreen] = useState<Screen>("setup");
  const [connection, setConnection] = useState<Connection | null>(null);
  const [state, setState] = useState<DeckState | null>(null);
  const [offline, setOffline] = useState(false);
  const [message, setMessage] = useState("");
  const [pressedId, setPressedId] = useState<string | null>(null);

  const [hostInput, setHostInput] = useState("");
  const [portInput, setPortInput] = useState("8787");
  const [codeInput, setCodeInput] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [setupError, setSetupError] = useState("");

  const connectionRef = useRef<Connection | null>(null);
  connectionRef.current = connection;

  const goSetup = useCallback((error?: string) => {
    const current = connectionRef.current;
    if (current) {
      setHostInput(current.host);
      setPortInput(String(current.port));
    }
    clearConnection();
    setConnection(null);
    setState(null);
    setScreen("setup");
    setMessage("");
    setSetupError(error ?? "");
  }, []);

  const loadDeck = useCallback(async (conn: Connection) => {
    const next = await fetchState(conn);
    setConnection(conn);
    setState(next);
    setOffline(false);
    setScreen("deck");
  }, []);

  // Reconnect automatically on launch.
  useEffect(() => {
    const saved = loadConnection();
    if (!saved) return;
    setHostInput(saved.host);
    setPortInput(String(saved.port));
    loadDeck(saved).catch(() => {
      setHostInput(saved.host);
      setPortInput(String(saved.port));
      setSetupError("Saved PC is not answering. Check WiFi and try again.");
    });
  }, [loadDeck]);

  // Keep the deck in sync with the PC (mode / profile / button edits).
  useEffect(() => {
    if (screen !== "deck" || !connection) return;
    let cancelled = false;
    const timer = setInterval(() => {
      fetchState(connection)
        .then((next) => {
          if (cancelled) return;
          setState(next);
          setOffline(false);
        })
        .catch(() => {
          if (!cancelled) setOffline(true);
        });
    }, 4000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [screen, connection]);

  const connect = async () => {
    const parsed = parseHostInput(hostInput);
    if (!parsed.host) {
      setSetupError("Enter the PC address shown in the app");
      return;
    }
    const port = parsed.port ?? (Number(portInput) || 8787);
    const code = codeInput.trim();

    setConnecting(true);
    setSetupError("");
    try {
      await ping(parsed.host, port);
      if (!code) {
        setSetupError("Enter the 6-digit code from the PC app");
        return;
      }
      const token = await pair(parsed.host, port, code);
      const conn: Connection = { host: parsed.host, port, token };
      saveConnection(conn);
      await loadDeck(conn);
      setMessage("");
    } catch (error) {
      setSetupError(error instanceof Error ? error.message : String(error));
    } finally {
      setConnecting(false);
    }
  };

  const press = async (buttonId: string) => {
    if (!connection || pressedId) return;
    setPressedId(buttonId);
    try {
      const result = await pressButton(connection, buttonId);
      if (result.state) setState(result.state);
      setOffline(false);
      setMessage(result.button ? `${result.button} ✓` : "Done");
    } catch (error) {
      const text = error instanceof Error ? error.message : String(error);
      if (text.toLowerCase().includes("not paired")) {
        goSetup("The pairing was reset on the PC. Pair again.");
      } else {
        setMessage(text);
      }
    } finally {
      setTimeout(() => setPressedId(null), 180);
    }
  };

  if (screen === "setup") {
    return (
      <div className="setup">
        <div className="setup-glow" aria-hidden="true" />
        <div className="setup-inner">
          <div className="brand">
            <span className="brand-mark">▦</span>
            <div>
              <div className="brand-title">AYMAN DECK</div>
              <div className="brand-sub">phone remote</div>
            </div>
          </div>

          <div className="setup-card">
            <div className="setup-head">
              <h1>Connect to your PC</h1>
              <p>
                Open <b>AYMAN MACRO DECK</b> on the computer, go to{" "}
                <b>Settings → Phone remote</b> and copy the address and code it
                shows.
              </p>
            </div>

            <div className="field-row">
              <label className="field field-grow">
                <span>PC ADDRESS</span>
                <input
                  value={hostInput}
                  onChange={(e) => setHostInput(e.target.value)}
                  placeholder="192.168.1.10"
                  inputMode="decimal"
                  autoCapitalize="none"
                  autoCorrect="off"
                  spellCheck={false}
                />
              </label>
              <label className="field field-port">
                <span>PORT</span>
                <input
                  value={portInput}
                  onChange={(e) => setPortInput(e.target.value)}
                  placeholder="8787"
                  inputMode="numeric"
                />
              </label>
            </div>

            <label className="field">
              <span>PAIRING CODE</span>
              <input
                value={codeInput}
                onChange={(e) =>
                  setCodeInput(e.target.value.replace(/\D/g, "").slice(0, 6))
                }
                placeholder="000000"
                inputMode="numeric"
                className="code-input"
              />
            </label>

            {setupError && (
              <div className="error" role="alert">
                <span className="error-mark">!</span>
                <span>{setupError}</span>
              </div>
            )}

            <button
              className="primary"
              onClick={() => void connect()}
              disabled={connecting}
            >
              {connecting ? (
                <>
                  <Spinner /> Connecting…
                </>
              ) : (
                "Connect to PC"
              )}
            </button>

            <p className="fineprint">
              <WifiIcon />
              Same WiFi as the PC. The connection is remembered for next time.
            </p>
          </div>
        </div>
      </div>
    );
  }

  const buttons = state?.buttons ?? [];

  return (
    <div className="deck">
      <header className="deck-header">
        <div className="deck-title">
          <span className="brand-mark small">▦</span>
          <div className="deck-title-text">
            <div className="profile-name">{state?.profileName ?? "Deck"}</div>
            <div className="deck-meta">
              <span className={`status-dot ${offline ? "off" : "on"}`} />
              <span className="deck-address">
                {offline
                  ? "Offline"
                  : connection
                    ? `${connection.host}:${connection.port}`
                    : "Connected"}
              </span>
              <span className={`mode-pill ${state?.gamingMode ? "gaming" : ""}`}>
                {state?.mode ?? "Normal"} mode
              </span>
            </div>
          </div>
        </div>
        <div className="header-actions">
          <button
            className="icon-btn"
            title="Refresh"
            aria-label="Refresh deck"
            onClick={() => {
              if (!connection) return;
              fetchState(connection)
                .then((next) => {
                  setState(next);
                  setOffline(false);
                })
                .catch(() => setOffline(true));
            }}
          >
            <RefreshIcon />
          </button>
          <button
            className="icon-btn"
            title="Settings"
            aria-label="Connection settings"
            onClick={() => goSetup()}
          >
            <SettingsIcon />
          </button>
        </div>
      </header>

      {offline && (
        <div className="offline" role="status">
          <span className="status-dot off" />
          PC not reachable — reconnecting…
        </div>
      )}

      <main className="grid">
        {buttons.map((button) => (
          <button
            key={button.id}
            className={`tile ${button.enabled ? "" : "disabled"} ${
              pressedId === button.id ? "pressed" : ""
            }`}
            aria-label={button.name}
            onClick={() => void press(button.id)}
          >
            <span className="tile-key">{keyLabel(button.key)}</span>
            <span className="tile-icon">{button.icon || "▢"}</span>
            <span className="tile-name">{button.name}</span>
            {button.actionCount > 1 && (
              <span className="tile-count">{button.actionCount}</span>
            )}
          </button>
        ))}
        {buttons.length === 0 && (
          <div className="empty">No buttons in this profile yet.</div>
        )}
      </main>

      <footer className="deck-footer">
        <span className={`hint ${message ? "active" : ""}`}>
          {message || "Tap a button to run it on the PC"}
        </span>
        <span className="footer-count">
          {buttons.length} {buttons.length === 1 ? "key" : "keys"}
        </span>
      </footer>
    </div>
  );
}
