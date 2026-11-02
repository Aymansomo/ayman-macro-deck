import { useState, useRef, useEffect } from "react";
import { useAppStore } from "../store";

export function ProfileSelector() {
  const config = useAppStore((s) => s.config);
  const setActiveProfile = useAppStore((s) => s.setActiveProfile);
  const saveConfig = useAppStore((s) => s.saveConfig);
  const [open, setOpen] = useState(false);
  const [openUp, setOpenUp] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  const toggle = () => {
    setOpen((prev) => {
      if (prev) return false;
      requestAnimationFrame(() => {
        const trigger = ref.current;
        if (trigger) {
          const rect = trigger.getBoundingClientRect();
          const estHeight = menuRef.current
            ? menuRef.current.offsetHeight
            : config.profiles.length * 36 + 8;
          setOpenUp(rect.bottom + estHeight + 8 > window.innerHeight);
        }
      });
      return true;
    });
  };

  return (
    <div ref={ref} className="relative">
      <button
        onClick={toggle}
        className="flex items-center gap-2 px-4 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors text-sm font-medium cursor-pointer"
      >
        <span className="text-[var(--color-text-primary)]">
          {activeProfile?.name ?? "Select Profile"}
        </span>
        <svg
          className={`w-4 h-4 text-[var(--color-text-muted)] transition-transform ${open ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </button>

      {open && (
        <div
          ref={menuRef}
          className={`absolute ${openUp ? "bottom-full left-0 mb-1" : "top-full left-0 mt-1"} w-56 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] shadow-xl z-50 py-1`}
        >
          {config.profiles.map((profile) => (
            <button
              key={profile.id}
              onClick={() => {
                setActiveProfile(profile.id);
                saveConfig();
                setOpen(false);
              }}
              className={`w-full text-left px-4 py-2 text-sm transition-colors cursor-pointer ${
                profile.id === config.activeProfileId
                  ? "bg-[var(--color-accent)] text-white"
                  : "text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)]"
              }`}
            >
              {profile.name}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
