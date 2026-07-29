import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppState } from "../App";

interface FreenetPageProps {
  appState: AppState;
  setAppState: React.Dispatch<React.SetStateAction<AppState>>;
}

export default function FreenetPage({ appState, setAppState }: FreenetPageProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [showBatModal, setShowBatModal] = useState(false);
  const [pendingStart, setPendingStart] = useState(false);
  const [toggling, setToggling] = useState(false);

  const isAllRunning = appState.tg_proxy.running && appState.zapret.running;
  const bothInstalled = appState.tg_proxy.installed && appState.zapret.installed;

  useEffect(() => {
    checkInstalledVersions();

    // Poll status every 3 seconds
    const interval = setInterval(pollStatus, 3000);
    return () => clearInterval(interval);
  }, []);

  const pollStatus = async () => {
    try {
      const status = await invoke<AppState>("get_all_status");
      setAppState((prev) => ({
        tg_proxy: { ...prev.tg_proxy, running: status.tg_proxy.running },
        zapret: { ...prev.zapret, running: status.zapret.running },
      }));
    } catch (e) {
      console.warn("Status poll failed:", e);
    }
  };

  const checkInstalledVersions = async () => {
    try {
      const [tgInstalled, zapretInstalled] = await Promise.all([
        invoke<boolean>("is_installed", { service: "tg-ws-proxy" }),
        invoke<boolean>("is_installed", { service: "zapret-discord-youtube" }),
      ]);

      const [tgVersion, zapretVersion, zapretLatest, tgLatest] = await Promise.all([
        tgInstalled ? invoke<string | null>("get_installed_version", { service: "tg-ws-proxy" }).catch(() => null) : Promise.resolve(null),
        zapretInstalled ? invoke<string | null>("get_installed_version", { service: "zapret-discord-youtube" }).catch(() => null) : Promise.resolve(null),
        invoke<string>("check_version", { service: "zapret-discord-youtube" }).catch((e) => { console.warn("Failed to check zapret version:", e); return null; }),
        invoke<string>("check_version", { service: "tg-ws-proxy" }).catch((e) => { console.warn("Failed to check tg version:", e); return null; }),
      ]);

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

  const handleToggle = async () => {
    if (toggling) return;
    setToggling(true);

    try {
      if (isAllRunning) {
        await invoke("stop_service", { service: "tg_proxy" });
        await invoke("stop_service", { service: "zapret" });
        setAppState((prev) => ({
          tg_proxy: { ...prev.tg_proxy, running: false },
          zapret: { ...prev.zapret, running: false },
        }));
      } else {
        if (!bothInstalled) return;

        const hasBatFile = await invoke<string | null>("load_config_value", { key: "zapret_bat_file" });
        if (!hasBatFile) {
          setShowBatModal(true);
          setPendingStart(true);
          setToggling(false);
          return;
        }

        await invoke("start_service", { service: "tg_proxy" });
        await invoke("start_service", { service: "zapret" });
        setAppState((prev) => ({
          tg_proxy: { ...prev.tg_proxy, running: true },
          zapret: { ...prev.zapret, running: true },
        }));
      }
    } catch (e) {
      console.error("Toggle failed:", e);
    } finally {
      setToggling(false);
    }
  };

  const handleBatSelected = async (batFile: string) => {
    await invoke("save_config_value", { key: "zapret_bat_file", value: batFile });
    setShowBatModal(false);
    if (pendingStart) {
      setPendingStart(false);
      setToggling(true);
      try {
        await invoke("start_service", { service: "tg_proxy" });
        await invoke("start_service", { service: "zapret" });
        setAppState((prev) => ({
          tg_proxy: { ...prev.tg_proxy, running: true },
          zapret: { ...prev.zapret, running: true },
        }));
      } catch (e) {
        console.error("Start failed:", e);
      } finally {
        setToggling(false);
      }
    }
  };

  const getStatusText = () => {
    if (!bothInstalled) return "DOWNLOAD REQUIRED";
    if (toggling) return "INITIALIZING...";
    if (isAllRunning) return "SYSTEM ACTIVE";
    return "SYSTEM STANDBY";
  };

  const getStatusDescription = () => {
    if (!bothInstalled) return "Go to DOWNLOADS tab to install required services.";
    if (toggling) return "Starting services, please wait...";
    if (isAllRunning) return "All services are running. Your connection is secured and optimized.";
    return "Initiate core connection protocol to access secure mainframe environments.";
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center relative w-full">
      <div className="absolute inset-0 bg-gradient-to-b from-primary/5 via-transparent to-transparent pointer-events-none" />

      {/* Power Button */}
      <div
        className={`relative group w-80 h-80 flex items-center justify-center ${bothInstalled ? "cursor-pointer" : "cursor-not-allowed"}`}
        onClick={bothInstalled ? handleToggle : undefined}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        {/* Outer Glow - pulses when running */}
        <div
          className={`absolute inset-0 rounded-full blur-3xl transition-all duration-1000 ${
            isAllRunning
              ? "bg-green-500/25 animate-pulse"
              : bothInstalled && isHovered
              ? "bg-primary-container/30"
              : "bg-primary-container/15"
          }`}
        />

        {/* Ring */}
        <div
          className={`absolute inset-6 rounded-full border bg-surface/50 liquid-blur shadow-[inset_0_4px_40px_rgba(255,255,255,0.08)] transition-all duration-500 ${
            isAllRunning
              ? "border-green-400/30 shadow-[0_0_30px_rgba(74,222,128,0.2)]"
              : bothInstalled && isHovered
              ? "border-primary/30"
              : "border-white/10"
          }`}
        />

        {/* Rotating Ring Decoration - only when running */}
        <svg
          className={`absolute inset-0 w-full h-full pointer-events-none transition-opacity duration-700 ${
            isAllRunning ? "animate-spin-slow opacity-40" : "opacity-0"
          }`}
          viewBox="0 0 320 320"
        >
          <defs>
            <linearGradient id="ringGrad" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="#22c55e" stopOpacity="0" />
              <stop offset="50%" stopColor="#22c55e" stopOpacity="1" />
              <stop offset="100%" stopColor="#22c55e" stopOpacity="0" />
            </linearGradient>
          </defs>
          <circle
            cx="160"
            cy="160"
            r="155"
            fill="none"
            stroke="url(#ringGrad)"
            strokeWidth="2"
            strokeDasharray="20 10"
          />
        </svg>

        {/* Inner rotating ring - counter direction */}
        <svg
          className={`absolute inset-0 w-full h-full pointer-events-none transition-opacity duration-700 ${
            isAllRunning ? "opacity-20" : "opacity-0"
          }`}
          viewBox="0 0 320 320"
          style={{ animation: "spin-slow 30s linear infinite reverse" }}
        >
          <circle
            cx="160"
            cy="160"
            r="140"
            fill="none"
            stroke="#22c55e"
            strokeWidth="1"
            strokeDasharray="5 15"
          />
        </svg>

        {/* Central Button */}
        <button
          className={`relative w-48 h-48 rounded-full liquid-blur glass-refraction flex items-center justify-center transition-all duration-500 z-10 overflow-hidden border border-white/20 ${
            bothInstalled && isHovered && !toggling ? "scale-105" : "scale-100"
          } ${
            isAllRunning
              ? "liquid-pulse-active bg-gradient-to-b from-green-900/40 to-surface/95"
              : bothInstalled
              ? "liquid-pulse bg-gradient-to-b from-surface-bright/50 to-surface/95"
              : "bg-surface/80 opacity-50"
          }`}
          disabled={!bothInstalled || toggling}
        >
          {/* Reflection Overlay */}
          <div className={`absolute inset-0 bg-gradient-to-tr from-transparent via-white/15 to-transparent transition-opacity duration-500 ${isHovered ? "opacity-100" : "opacity-0"}`} />

          {/* Loading spinner when toggling */}
          {toggling ? (
            <div className="w-16 h-16 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
          ) : (
            <span
              className={`material-symbols-outlined text-[84px] font-light transition-all duration-500 ${
                isAllRunning
                  ? "text-green-400 drop-shadow-[0_0_30px_rgba(74,222,128,0.8)]"
                  : bothInstalled
                  ? "text-primary drop-shadow-[0_0_25px_rgba(188,19,254,0.6)] group-hover:drop-shadow-[0_0_40px_rgba(188,19,254,0.9)]"
                  : "text-outline/40"
              }`}
            >
              power_settings_new
            </span>
          )}
        </button>
      </div>

      {/* Status Text */}
      <div className="mt-16 text-center z-10 px-12">
        <h2
          className={`font-headline text-headline-sm mb-4 tracking-[0.2em] font-bold uppercase drop-shadow-md transition-all duration-500 ${
            isAllRunning
              ? "text-green-300 drop-shadow-[0_0_15px_rgba(74,222,128,0.5)]"
              : toggling
              ? "text-primary animate-pulse"
              : "text-on-surface"
          }`}
        >
          {getStatusText()}
        </h2>
        <p className="font-body text-body-md text-on-surface-variant max-w-[480px] mx-auto leading-relaxed opacity-90">
          {getStatusDescription()}
        </p>

        {/* Service Indicators */}
        <div className="mt-8 flex gap-8 justify-center">
          <ServiceIndicator
            name="TG-WS-PROXY"
            installed={appState.tg_proxy.installed}
            running={appState.tg_proxy.running}
          />
          <ServiceIndicator
            name="ZAPRET"
            installed={appState.zapret.installed}
            running={appState.zapret.running}
          />
        </div>
      </div>

      {showBatModal && (
        <BatFileModal onSelect={handleBatSelected} onClose={() => { setShowBatModal(false); setPendingStart(false); }} />
      )}
    </div>
  );
}

function ServiceIndicator({
  name,
  installed,
  running,
}: {
  name: string;
  installed: boolean;
  running: boolean;
}) {
  return (
    <div className="flex items-center gap-2.5">
      <div
        className={`w-2.5 h-2.5 rounded-full transition-all duration-500 ${
          running
            ? "bg-green-400 shadow-[0_0_10px_rgba(74,222,128,0.8)] animate-pulse"
            : installed
            ? "bg-outline/50"
            : "bg-red-400/50"
        }`}
      />
      <span
        className={`font-label text-[11px] tracking-[0.15em] font-bold uppercase transition-colors duration-500 ${
          running ? "text-green-400/80" : installed ? "text-outline/60" : "text-red-400/60"
        }`}
      >
        {name}
        {!installed ? " (NOT INSTALLED)" : running ? " (ACTIVE)" : ""}
      </span>
    </div>
  );
}

function BatFileModal({
  onSelect,
  onClose,
}: {
  onSelect: (file: string) => void;
  onClose: () => void;
}) {
  const [batFiles, setBatFiles] = useState<string[]>([]);
  const [selected, setSelected] = useState<string>("");

  useEffect(() => {
    invoke<string[]>("list_bat_files").then((files) => {
      setBatFiles(files);
      if (files.length > 0) setSelected(files[0]);
    });
  }, []);

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="glass-card rounded-2xl p-6 border border-white/15 bg-surface/80 liquid-blur w-[420px] shadow-2xl">
        <h3 className="font-headline text-headline-sm text-on-surface tracking-[0.15em] font-bold uppercase mb-2 text-center">
          SELECT STRATEGY
        </h3>
        <p className="font-body text-[12px] text-on-surface-variant opacity-70 text-center mb-6">
          Choose which bat file zapret should use. You can change this later in Settings.
        </p>

        {batFiles.length === 0 ? (
          <p className="text-center text-outline/50 text-[12px] mb-6">
            No bat files found. Make sure zapret is downloaded.
          </p>
        ) : (
          <div className="space-y-2 mb-6 max-h-[240px] overflow-y-auto pr-1">
            {batFiles.map((file) => (
              <button
                key={file}
                onClick={() => setSelected(file)}
                className={`w-full text-left px-4 py-3 rounded-xl border font-label text-[13px] tracking-wide transition-all duration-200 ${
                  selected === file
                    ? "bg-primary/20 border-primary/40 text-primary"
                    : "bg-surface-container-high/40 border-white/8 text-on-surface-variant hover:border-white/15"
                }`}
              >
                {file.replace(".bat", "")}
              </button>
            ))}
          </div>
        )}

        <div className="flex gap-3">
          <button
            onClick={onClose}
            className="flex-1 py-2.5 rounded-xl bg-surface-container-high/60 border border-white/10 text-on-surface-variant font-label text-[12px] tracking-wider font-bold transition-all duration-300 hover:bg-surface-container-high/80"
          >
            CANCEL
          </button>
          <button
            onClick={() => selected && onSelect(selected)}
            disabled={!selected}
            className="flex-1 py-2.5 rounded-xl bg-primary/20 border border-primary/30 text-primary font-label text-[12px] tracking-wider font-bold transition-all duration-300 hover:bg-primary/30 active:scale-[0.98] disabled:opacity-40"
          >
            CONFIRM
          </button>
        </div>
      </div>
    </div>
  );
}
