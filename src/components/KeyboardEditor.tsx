import { useEffect, useState } from "react";
import { useAppStore } from "../store";
import { LAYOUT_OPTIONS, allKeysForLayout, keyLabel } from "../lib/keyboardLayouts";
import type { Profile } from "../types";
import { KeyboardView } from "./KeyboardView";

interface Props {
  profile: Profile;
  selectedButtonId: string | null;
  activeKeys: Set<string>;
  runningButtonId: string | null;
  onSelectButton: (buttonId: string) => void;
}

export function KeyboardEditor({
  profile,
  selectedButtonId,
  activeKeys,
  runningButtonId,
  onSelectButton,
}: Props) {
  const setProfileLayout = useAppStore((s) => s.setProfileLayout);
  const ensureButtonForPhysicalKey = useAppStore(
    (s) => s.ensureButtonForPhysicalKey,
  );
  const addPage = useAppStore((s) => s.addPage);
  const renamePage = useAppStore((s) => s.renamePage);
  const deletePage = useAppStore((s) => s.deletePage);

  // Which page of the profile is being edited; null is the main page.
  const [pageId, setPageId] = useState<string | null>(null);
  const [newPageName, setNewPageName] = useState("");
  const [creatingPage, setCreatingPage] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  const layout = profile.layout ?? "numpad";
  const pages = profile.pages ?? [];
  const page = pages.find((candidate) => candidate.id === pageId) ?? null;
  const buttons = page ? page.buttons : profile.buttons;

  useEffect(() => {
    if (pageId && !pages.some((candidate) => candidate.id === pageId)) {
      setPageId(null);
    }
  }, [pages, pageId]);

  const visibleIds = new Set(allKeysForLayout(layout).map((key) => key.id));
  const offLayoutButtons = buttons.filter(
    (button) =>
      Boolean(button.physicalKeyId) && !visibleIds.has(button.physicalKeyId!),
  );

  const handleSelectKey = (physicalKeyId: string) => {
    const buttonId = ensureButtonForPhysicalKey(
      profile.id,
      physicalKeyId,
      page?.id ?? null,
    );
    if (buttonId) onSelectButton(buttonId);
  };

  const createPage = () => {
    const id = addPage(profile.id, newPageName);
    setNewPageName("");
    setCreatingPage(false);
    setPageId(id);
  };

  const configuredCount = buttons.filter(
    (button) => button.actions.length > 0,
  ).length;
  const actionCount = buttons.reduce(
    (total, button) => total + button.actions.length,
    0,
  );

  return (
    <div className="flex-1 min-w-0 flex flex-col overflow-hidden">
      <div className="keyboard-toolbar">
        <div>
          <div className="text-xs font-semibold text-[var(--color-text-primary)]">
            {profile.name}
            {page ? ` · ${page.name}` : ""} · Keyboard Layout
          </div>
          <div className="text-[10px] text-[var(--color-text-muted)] mt-0.5">
            {configuredCount} configured key{configuredCount === 1 ? "" : "s"} ·{" "}
            {actionCount} action{actionCount === 1 ? "" : "s"} · click any key to
            edit or assign
          </div>
        </div>
        <div className="layout-switcher">
          {LAYOUT_OPTIONS.map((option) => (
            <button
              key={option.value}
              type="button"
              onClick={() => setProfileLayout(profile.id, option.value)}
              title={option.description}
              className={`px-3 py-1.5 rounded-md text-[11px] transition-colors cursor-pointer ${
                layout === option.value
                  ? "bg-[var(--color-accent)] text-white font-medium"
                  : "text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)]"
              }`}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      {/* Pages (folders): each one has its own key assignments. */}
      <div className="flex items-center gap-1.5 px-4 py-2 border-b border-[var(--color-border)] overflow-x-auto shrink-0">
        <span className="text-[9px] tracking-wider text-[var(--color-text-muted)] mr-1 shrink-0">
          PAGES
        </span>
        <button
          type="button"
          onClick={() => setPageId(null)}
          className={`px-2.5 py-1 rounded-md text-[11px] cursor-pointer transition-colors shrink-0 ${
            pageId === null
              ? "bg-[var(--color-accent)] text-white font-medium"
              : "text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-card-hover)]"
          }`}
        >
          Main
        </button>

        {pages.map((candidate) =>
          renaming === candidate.id ? (
            <input
              key={candidate.id}
              autoFocus
              value={renameValue}
              onChange={(event) => setRenameValue(event.target.value)}
              onBlur={() => {
                renamePage(profile.id, candidate.id, renameValue);
                setRenaming(null);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  renamePage(profile.id, candidate.id, renameValue);
                  setRenaming(null);
                }
                if (event.key === "Escape") setRenaming(null);
              }}
              className="px-2 py-1 rounded-md text-[11px] bg-[var(--color-bg-card)] border border-[var(--color-accent)] text-[var(--color-text-primary)] outline-none w-28 shrink-0"
            />
          ) : (
            <span key={candidate.id} className="flex items-center shrink-0">
              <button
                type="button"
                onClick={() => setPageId(candidate.id)}
                onDoubleClick={() => {
                  setRenaming(candidate.id);
                  setRenameValue(candidate.name);
                }}
                title="Double-click to rename"
                className={`px-2.5 py-1 rounded-md text-[11px] cursor-pointer transition-colors ${
                  pageId === candidate.id
                    ? "bg-[var(--color-accent)] text-white font-medium"
                    : "text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-card-hover)]"
                }`}
              >
                {candidate.name}
              </button>
              <button
                type="button"
                onClick={() => {
                  deletePage(profile.id, candidate.id);
                  if (pageId === candidate.id) setPageId(null);
                }}
                title={`Delete page "${candidate.name}"`}
                className="ml-0.5 w-4 h-4 flex items-center justify-center rounded text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-danger)] cursor-pointer"
              >
                ✕
              </button>
            </span>
          ),
        )}

        {creatingPage ? (
          <input
            autoFocus
            value={newPageName}
            onChange={(event) => setNewPageName(event.target.value)}
            onBlur={createPage}
            onKeyDown={(event) => {
              if (event.key === "Enter") createPage();
              if (event.key === "Escape") {
                setNewPageName("");
                setCreatingPage(false);
              }
            }}
            placeholder="Page name…"
            className="px-2 py-1 rounded-md text-[11px] bg-[var(--color-bg-card)] border border-[var(--color-accent)] text-[var(--color-text-primary)] outline-none w-28 shrink-0"
          />
        ) : (
          <button
            type="button"
            onClick={() => setCreatingPage(true)}
            className="px-2.5 py-1 rounded-md text-[11px] text-[var(--color-accent)] hover:bg-[var(--color-bg-card-hover)] cursor-pointer shrink-0"
          >
            + Page
          </button>
        )}
      </div>

      <div className="keyboard-canvas">
        <KeyboardView
          layout={layout}
          buttons={buttons}
          selectedButtonId={selectedButtonId}
          activeKeys={activeKeys}
          runningButtonId={runningButtonId}
          onSelectKey={handleSelectKey}
        />

        {offLayoutButtons.length > 0 && (
          <div className="mt-8 mx-auto max-w-3xl rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-card)] p-3">
            <div className="text-[9px] tracking-wider text-[var(--color-text-muted)] mb-2">
              ASSIGNED KEYS OUTSIDE THIS LAYOUT (PRESERVED)
            </div>
            <div className="flex flex-wrap gap-2">
              {offLayoutButtons.map((button) => (
                <button
                  key={button.id}
                  type="button"
                  onClick={() => onSelectButton(button.id)}
                  className="px-2 py-1 rounded-md text-[10px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:border-[var(--color-accent)] cursor-pointer"
                >
                  {keyLabel(button.physicalKeyId!)} · {button.name}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
