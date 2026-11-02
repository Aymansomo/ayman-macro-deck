import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { MacroButton as MacroButtonType } from "../types";

interface Props {
  button: MacroButtonType;
  isSelected: boolean;
  onSelect: (buttonId: string) => void;
}

export function MacroButton({ button, isSelected, onSelect }: Props) {
  const isSide = button.key === 10 || button.key === 11;
  const [imageFailed, setImageFailed] = useState(false);
  const showImage = Boolean(button.imagePath) && !imageFailed;

  useEffect(() => {
    setImageFailed(false);
  }, [button.imagePath]);

  return (
    <button
      onClick={() => onSelect(button.id)}
      className={`group relative flex flex-col items-center justify-center rounded-xl border transition-all duration-150 cursor-pointer select-none aspect-square overflow-hidden ${
        isSelected
          ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)] shadow-[0_0_24px_var(--color-accent-glow)]"
          : button.enabled
            ? "bg-[var(--color-bg-card)] border-[var(--color-border)] hover:border-[var(--color-border-hover)] hover:bg-[var(--color-bg-card-hover)]"
            : "bg-[var(--color-bg-secondary)] border-[var(--color-border)] opacity-40"
      }`}
    >
      <div className="absolute top-2 left-2 text-[10px] font-mono text-[var(--color-text-muted)]">
        {isSide ? (button.key === 10 ? "/" : "*") : button.key}
      </div>

      {button.actions.length > 0 && (
        <div className="absolute top-2 right-2 text-[9px] font-mono text-[var(--color-accent)] bg-[var(--color-accent)]/10 rounded-full w-4 h-4 flex items-center justify-center">
          {button.actions.length}
        </div>
      )}

      {!button.enabled && (
        <div className="absolute top-2 right-2 text-[9px] text-[var(--color-danger)]">
          OFF
        </div>
      )}

      <div
        className={`mb-2 flex items-center justify-center ${
          isSide ? "w-10 h-10" : "w-14 h-14"
        }`}
      >
        {showImage ? (
          <img
            src={convertFileSrc(button.imagePath!)}
            alt={button.name}
            draggable={false}
            className="max-w-full max-h-full object-contain"
            onError={() => setImageFailed(true)}
          />
        ) : (
          <span className={isSide ? "text-2xl" : "text-3xl"}>
            {button.icon || "⬜"}
          </span>
        )}
      </div>

      <div className="text-xs font-medium text-[var(--color-text-primary)] text-center px-2 leading-tight">
        {button.name}
      </div>

      <div className="absolute bottom-1.5 left-0 right-0 text-center text-[9px] text-[var(--color-text-muted)] opacity-0 group-hover:opacity-100 transition-opacity">
        click to edit
      </div>
    </button>
  );
}
