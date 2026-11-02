import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Status =
  | "idle"
  | "checking"
  | "latest"
  | "available"
  | "downloading"
  | "error";

export function UpdateButton() {
  const [version, setVersion] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [update, setUpdate] = useState<Update | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [message, setMessage] = useState("");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => {});
  }, []);

  const checkForUpdate = async () => {
    setStatus("checking");
    setMessage("");
    setProgress(null);
    try {
      const found = await check();
      if (found) {
        setUpdate(found);
        setStatus("available");
        setMessage(`Version ${found.version} is available`);
      } else {
        setUpdate(null);
        setStatus("latest");
        setMessage("You are on the latest version");
      }
    } catch (err) {
      setStatus("error");
      setMessage(
        typeof err === "string" ? err : "Could not check for updates",
      );
    }
  };

  const install = async () => {
    if (!update) return;
    setStatus("downloading");
    setProgress(0);
    setMessage("");
    try {
      let total = 0;
      let downloaded = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          setProgress(0);
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          setProgress(total > 0 ? Math.round((downloaded / total) * 100) : null);
        } else if (event.event === "Finished") {
          setProgress(100);
        }
      });
      await relaunch();
    } catch (err) {
      setStatus("error");
      setMessage(
        typeof err === "string" ? err : "Update failed to install",
      );
    }
  };

  const label =
    status === "checking"
      ? "Checking…"
      : status === "available"
        ? `Update ${update ? update.version : ""}`
        : status === "downloading"
          ? progress === null
            ? "Installing…"
            : `${progress}%`
          : status === "latest"
            ? "Up to date"
            : status === "error"
              ? "Update failed"
              : version
                ? `v${version}`
                : "Update";

  const accent =
    status === "available" || status === "downloading"
      ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)]/40 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25"
      : status === "error"
        ? "bg-[var(--color-danger)]/10 border-[var(--color-danger)]/30 text-[var(--color-danger)]"
        : status === "latest"
          ? "bg-[var(--color-success)]/10 border-[var(--color-success)]/30 text-[var(--color-success)]"
          : "bg-[var(--color-bg-card)] border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-hover)]";

  return (
    <button
      type="button"
      onClick={() => {
        if (status === "available") void install();
        else if (status !== "downloading" && status !== "checking")
          void checkForUpdate();
      }}
      disabled={status === "downloading" || status === "checking"}
      title={message || "Check for updates"}
      className={`px-2.5 py-1 rounded-lg border text-[11px] font-medium font-mono transition-colors cursor-pointer disabled:cursor-wait ${accent}`}
    >
      {label}
    </button>
  );
}
