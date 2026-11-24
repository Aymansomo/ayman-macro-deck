import { useEffect } from "react";

export interface ProfileToastData {
  id: string;
  name: string;
}

/** Short notice shown whenever the active profile changes, including the
 *  automatic switch for a linked app, so the deck never changes silently. */
export function ProfileToast({
  toast,
  onDone,
}: {
  toast: ProfileToastData | null;
  onDone: () => void;
}) {
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(onDone, 2400);
    return () => clearTimeout(timer);
  }, [toast, onDone]);

  if (!toast) return null;

  return (
    <div className="profile-toast" role="status" aria-live="polite">
      <span className="profile-toast-label">PROFILE</span>
      <span className="profile-toast-name">{toast.name}</span>
    </div>
  );
}
