import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppState } from "../App";

interface FreenetPageProps {
  appState: AppState;
  setAppState: React.Dispatch<React.SetStateAction<AppState>>;
}

interface BypassServiceInfo {
  key: string;
  name: string;
  description: string;
  installed: boolean;
  running: boolean;
  exclusive: boolean;
  installed_version: string | null;
  latest_version: string | null;
}

export default function FreenetPage({ appState, setAppState }: FreenetPageProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [toggling, setToggling] = useState(false);
  const [bypasses, setBypasses] = useState<BypassServiceInfo[]>([]);
  const [selectedBypass, setSelectedBypass] = useState<string | null>(null);
  const [activeBypass, setActiveBypass] = useState<string | null>(null);
  const pendingStart = useRef(false);
  const [showBatModal, setShowBatModal] = useState(false);

  const tgRunning = appState.tg_proxy.running;
  const activeKey = activeBypass ?? selectedBypass;
  const anyRunning = tgRunning || activeBypass !== null;
  const bypassInstalled = bypasses.find((b) => b.key === activeKey)?.installed ?? false;
  const anythingInstalled =
    tgRunning || bypasses.some((b) => b.installed) || appState.tg_proxy.installed;

  useEffect(() => {
    checkInstalledVersions();
    loadBypasses();
    pollStatus();

    const interval = setInterval(() => {
      pollStatus();
      loadBypasses();
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  const loadBypasses = async () => {
    try {
      const [list, active, saved] = await Promise.all([
        invoke<BypassServiceInfo[]>("get_bypass_services"),
        invoke<string | null>("get_active_bypass"),
        invoke<string | null>("load_config_value", { key: "selected_bypass" }),
      ]);
      setBypasses(list);
      setActiveBypass(active);
      setSelectedBypass((prev) => prev ?? (saved ?? "zapret"));
    } catch (e) {
      console.warn("Failed to load bypasses:", e);
    }
  };

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
      const [tgInstalled] = await Promise.all([
        invoke<boolean>("is_installed", { service: "tg-ws-proxy" }),
      ]);
      const tgVersion = tgInstalled
        ? await invoke<string | null>("get_installed_version", { service: "tg-ws-proxy" }).catch(() => null)
        : null;
      setAppState((prev) => ({
        tg_proxy: {
          ...prev.tg_proxy,
          installed: tgInstalled,
          installedVersion: tgVersion,
        },
        zapret: prev.zapret,
      }));
    } catch (e) {
      console.error("Failed to check versions:", e);
    }
  };

  const startSelected = async () => {
    if (!selectedBypass) return;
    await invoke("start_bypass", { key: selectedBypass });
    setActiveBypass(selectedBypass);
    if (appState.tg_proxy.installed) {
      try {
        await invoke("start_bypass", { key: "tg-ws-proxy" });
        setAppState((prev) => ({ ...prev, tg_proxy: { ...prev.tg_proxy, running: true } }));
      } catch (e) {
        console.warn("tg-ws-proxy did not start:", e);
      }
    }
  };

  const handleToggle = async () => {
    if (toggling) return;
    setToggling(true);

    try {
      if (anyRunning) {
        await invoke("stop_all_services");
        for (const b of bypasses) {
          if (b.running) await invoke("stop_bypass", { key: b.key });
        }
        setAppState((prev) => ({
          tg_proxy: { ...prev.tg_proxy, running: false },
          zapret: { ...prev.zapret, running: false },
        }));
        setActiveBypass(null);
      } else {
        if (!selectedBypass) return;

        if (selectedBypass === "zapret") {
          const hasBatFile = await invoke<string | null>("load_config_value", { key: "zapret_bat_file" });
          if (!hasBatFile) {
            setShowBatModal(true);
            pendingStart.current = true;
            setToggling(false);
            return;
          }
        }

        await startSelected();
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
    if (pendingStart.current) {
      pendingStart.current = false;
      setToggling(true);
      try {
        await startSelected();
      } catch (e) {
        console.error("Start failed:", e);
      } finally {
        setToggling(false);
      }
    }
  };

  const getStatusText = () => {
    if (!anythingInstalled) return "DOWNLOAD REQUIRED";
    if (toggling) return anyRunning ? "STOPPING..." : "STARTING...";
    if (anyRunning) return "SYSTEM ACTIVE — CLICK TO STOP";
    return "SYSTEM ON STANDBY";
  };

  const getStatusDescription = () => {
    if (!anythingInstalled)
      return "Go to the BYPASS tab to install and select a bypass tool.";
    if (toggling) return anyRunning ? "Stopping all services..." : "Starting services, please wait...";
    if (anyRunning) {
      const name = bypasses.find((b) => b.key === activeKey)?.name ?? "Bypass";
      const parts = [name];
      if (tgRunning) parts.push("tg-ws-proxy");
      return `${parts.join(" + ")} is running. Click the button to stop everything.`;
    }
    if (!selectedBypass) return "Select a bypass tool in the BYPASS tab, then start from here.";
    return "Start the selected bypass to access protected resources.";
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center relative w-full">
      <div className="absolute inset-0 bg-gradient-to-b from-primary/5 via-transparent to-transparent pointer-events-none" />

      {/* Power Button */}
      <div
        className={`relative group w-80 h-80 flex items-center justify-center ${anythingInstalled ? "cursor-pointer" : "cursor-not-allowed"}`}
        onClick={anythingInstalled ? handleToggle : undefined}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        {/* Outer Glow - pulses when running */}
        <div
          className={`absolute inset-0 rounded-full blur-3xl transition-all duration-1000 ${
            anyRunning
              ? "bg-green-500/25 animate-pulse"
              : anythingInstalled && isHovered
              ? "bg-primary-container/30"
              : "bg-primary-container/15"
          }`}
        />

        {/* Ring */}
        <div
          className={`absolute inset-6 rounded-full border bg-surface/50 liquid-blur shadow-[inset_0_4px_40px_rgba(255,255,255,0.08)] transition-all duration-500 ${
            anyRunning
              ? "border-green-400/30 shadow-[0_0_30px_rgba(74,222,128,0.2)]"
              : anythingInstalled && isHovered
              ? "border-primary/30"
              : "border-white/10"
          }`}
        />

        {/* Rotating Ring Decoration - only when running */}
        <svg
          className={`absolute inset-0 w-full h-full pointer-events-none transition-opacity duration-700 ${
            anyRunning ? "animate-spin-slow opacity-40" : "opacity-0"
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
            anyRunning ? "opacity-20" : "opacity-0"
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
            anythingInstalled && isHovered && !toggling ? "scale-105" : "scale-100"
          } ${
            anyRunning
              ? "liquid-pulse-active bg-gradient-to-b from-red-900/40 to-surface/95 border-red-400/40"
              : anythingInstalled
              ? "liquid-pulse bg-gradient-to-b from-surface-bright/50 to-surface/95"
              : "bg-surface/80 opacity-50"
          }`}
          disabled={!anythingInstalled || toggling}
        >
          {/* Reflection Overlay */}
          <div className={`absolute inset-0 bg-gradient-to-tr from-transparent via-white/15 to-transparent transition-opacity duration-500 ${isHovered ? "opacity-100" : "opacity-0"}`} />

          {/* Loading spinner when toggling */}
          {toggling ? (
            <div className="w-16 h-16 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
          ) : (
            <span
              className={`material-symbols-outlined text-[84px] font-light transition-all duration-500 ${
                anyRunning
                  ? "text-red-400 drop-shadow-[0_0_30px_rgba(248,113,113,0.8)]"
                  : anythingInstalled
                  ? "text-primary drop-shadow-[0_0_25px_rgba(188,19,254,0.6)] group-hover:drop-shadow-[0_0_40px_rgba(188,19,254,0.9)]"
                  : "text-outline/40"
              }`}
            >
              {anyRunning ? "power_off" : "power_settings_new"}
            </span>
          )}
        </button>
      </div>

      {/* Status Text */}
      <div className="mt-16 text-center z-10 px-12">
        <h2
          className={`font-headline text-headline-sm mb-4 tracking-[0.2em] font-bold uppercase drop-shadow-md transition-all duration-500 ${
            anyRunning
              ? "text-white drop-shadow-[0_0_15px_rgba(255,255,255,0.35)]"
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
        <div className="mt-8 flex flex-wrap gap-x-8 gap-y-3 justify-center">
          {bypasses
            .filter((b) => b.key !== "tg-ws-proxy" && (b.installed || b.running))
            .map((b) => (
              <ServiceIndicator
                key={b.key}
                name={b.name}
                installed={b.installed}
                running={b.running}
              />
            ))}
          {appState.tg_proxy.installed && (
            <ServiceIndicator
              name="TG-WS-PROXY"
              installed={appState.tg_proxy.installed}
              running={tgRunning}
            />
          )}
        </div>
      </div>

      {showBatModal && (
        <BatFileModal
          onSelect={handleBatSelected}
          onClose={() => {
            setShowBatModal(false);
            pendingStart.current = false;
          }}
        />
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
          Choose which bat file zapret should use. You can change this later in the BYPASS tab.
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
                    : "bg-surface-container-high/40 border-primary/15 text-on-surface-variant hover:border-primary/30"
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
            className="flex-1 py-2.5 rounded-xl bg-surface-container-high/60 border border-primary/15 text-on-surface-variant font-label text-[12px] tracking-wider font-bold transition-all duration-300 hover:bg-surface-container-high/80"
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
