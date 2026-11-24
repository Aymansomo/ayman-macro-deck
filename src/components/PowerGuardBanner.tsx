import { useEffect, useState } from "react";

export interface PowerWarning {
  actionType: string;
  label: string;
  /** Wall-clock time (ms) at which the armed state expires. */
  expiresAt: number;
}

interface Props {
  warning: PowerWarning | null;
  onExpired: () => void;
  /** Disarms the action in the engine and hides the banner. */
  onCancel: () => void;
}

/**
 * Shut Down / Restart only arm on the first press. This banner makes that
 * obvious with a live countdown, so a stray tap is never mistaken for a real
 * shutdown.
 */
export function PowerGuardBanner({ warning, onExpired, onCancel }: Props) {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!warning) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 100);
    return () => clearInterval(timer);
  }, [warning]);

  const remaining = warning ? Math.max(0, warning.expiresAt - now) : 0;

  useEffect(() => {
    if (warning && remaining <= 0) onExpired();
  }, [warning, remaining, onExpired]);

  if (!warning || remaining <= 0) return null;

  return (
    <div className="power-banner" role="alert" aria-live="assertive">
      <span className="power-banner-icon">⚠</span>
      <div className="power-banner-text">
        <strong>{warning.label} armed — nothing has happened yet.</strong>
        <span>Press the same key again to confirm, or cancel.</span>
      </div>
      <span className="power-banner-countdown">{(remaining / 1000).toFixed(1)}s</span>
      <button
        type="button"
        className="power-banner-cancel"
        onClick={onCancel}
        title="Cancel this Shut Down / Restart"
      >
        Cancel
      </button>
    </div>
  );
}
