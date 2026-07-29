import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppState, ServiceStatus } from "../App";

interface DownloadsPageProps {
  appState: AppState;
  setAppState: React.Dispatch<React.SetStateAction<AppState>>;
}

export default function DownloadsPage({ appState, setAppState }: DownloadsPageProps) {
  useEffect(() => {
    checkInstalledVersions();
  }, []);

  const checkInstalledVersions = async () => {
    try {
      const tgInstalled = await invoke<boolean>("is_installed", { service: "tg-ws-proxy" });
      const zapretInstalled = await invoke<boolean>("is_installed", { service: "zapret-discord-youtube" });
      const tgVersion = tgInstalled ? await invoke<string | null>("get_installed_version", { service: "tg-ws-proxy" }).catch(() => null) : null;
      const zapretVersion = zapretInstalled ? await invoke<string | null>("get_installed_version", { service: "zapret-discord-youtube" }).catch(() => null) : null;
      const zapretLatest = await invoke<string>("check_version", { service: "zapret-discord-youtube" }).catch(() => null);
      const tgLatest = await invoke<string>("check_version", { service: "tg-ws-proxy" }).catch(() => null);

      setAppState({
        tg_proxy: {
          name: "tg-ws-proxy",
          installed: tgInstalled,
          installedVersion: tgVersion,
          latestVersion: tgLatest,
          running: false,
        },
        zapret: {
          name: "zapret-discord-youtube",
          installed: zapretInstalled,
          installedVersion: zapretVersion,
          latestVersion: zapretLatest,
          running: false,
        },
      });
    } catch (e) {
      console.error("Failed to check versions:", e);
    }
  };

  const handleDownload = async (service: string) => {
    try {
      const result = await invoke<string>("download_service", { service });
      await checkInstalledVersions();
      return result;
    } catch (e) {
      throw e;
    }
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center relative w-full px-8">
      <div className="absolute inset-0 bg-gradient-to-b from-primary/5 via-transparent to-transparent pointer-events-none" />

      <div className="w-full max-w-[700px] z-10 space-y-6">
        <div className="text-center mb-6">
          <h2 className="font-headline text-headline-sm text-on-surface tracking-[0.2em] font-bold uppercase mb-2">
            DOWNLOADS
          </h2>
          <p className="font-body text-body-md text-on-surface-variant opacity-70">
            Download, update, and manage proxy services
          </p>
        </div>

        <ServiceCard
          service={appState.tg_proxy}
          onDownload={() => handleDownload("tg-ws-proxy")}
        />

        <ServiceCard
          service={appState.zapret}
          onDownload={() => handleDownload("zapret-discord-youtube")}
        />
      </div>
    </div>
  );
}

function ServiceCard({
  service,
  onDownload,
}: {
  service: ServiceStatus;
  onDownload: () => Promise<string>;
}) {
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState("");

  const needsUpdate =
    service.installed &&
    service.latestVersion &&
    service.installedVersion !== service.latestVersion;

  const handleDownload = async () => {
    setDownloading(true);
    setProgress("Downloading...");
    try {
      const result = await onDownload();
      setProgress(result);
      setTimeout(() => setProgress(""), 3000);
    } catch (e) {
      setProgress(`Error: ${e}`);
    } finally {
      setDownloading(false);
    }
  };

  return (
    <div className="glass-card rounded-2xl p-5 border border-white/10 bg-surface/30 liquid-blur">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <div
            className={`w-3 h-3 rounded-full transition-all duration-500 ${
              service.installed
                ? "bg-green-400 shadow-[0_0_12px_rgba(74,222,128,0.8)]"
                : "bg-outline/40"
            }`}
          />
          <h3 className="font-label text-[14px] text-on-surface tracking-[0.05em] font-bold uppercase">
            {service.name}
          </h3>
        </div>
        <div className="flex items-center gap-2">
          {service.installed && service.installedVersion && (
            <span className="font-label text-[11px] text-outline/60 tracking-wider">
              v{service.installedVersion}
            </span>
          )}
          {needsUpdate && (
            <span className="px-2 py-0.5 rounded-full bg-primary-container/30 text-primary text-[10px] font-bold tracking-wider">
              UPDATE AVAILABLE
            </span>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <button
          onClick={handleDownload}
          disabled={downloading}
          className={`flex-1 py-2.5 rounded-xl font-label text-[12px] tracking-wider font-bold transition-all duration-300 active:scale-[0.98] disabled:opacity-50 ${
            !service.installed
              ? "bg-primary/20 hover:bg-primary/30 border border-primary/30 text-primary"
              : needsUpdate
              ? "bg-primary/15 hover:bg-primary/25 border border-primary/20 text-primary"
              : "bg-surface-container-high/60 hover:bg-surface-container-high/80 border border-white/10 text-on-surface-variant"
          }`}
        >
          {downloading
            ? progress || "DOWNLOADING..."
            : !service.installed
            ? "DOWNLOAD"
            : needsUpdate
            ? "UPDATE"
            : "REINSTALL"}
        </button>
      </div>

      {downloading && (
        <div className="mt-3 h-1 bg-surface-container-high/60 rounded-full overflow-hidden">
          <div className="h-full bg-primary/60 rounded-full animate-pulse w-full" />
        </div>
      )}

      {!service.installed && (
        <p className="mt-3 font-body text-[11px] text-outline/50 text-center">
          Not installed. Click DOWNLOAD to get the latest version.
        </p>
      )}
    </div>
  );
}
