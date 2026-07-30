import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import TitleBar from "./components/TitleBar";
import NavBar from "./components/NavBar";
import StatusBar from "./components/StatusBar";
import FreenetPage from "./components/FreenetPage";
import DownloadsPage from "./components/DownloadsPage";
import SettingsPage from "./components/SettingsPage";

export type Page = "freenet" | "downloads" | "settings";

export interface ServiceStatus {
  name: string;
  installed: boolean;
  installedVersion: string | null;
  latestVersion: string | null;
  running: boolean;
}

export interface AppState {
  tg_proxy: ServiceStatus;
  zapret: ServiceStatus;
}

type LoaderPhase = "checking" | "updating" | "ready";

function App() {
  const [loaderPhase, setLoaderPhase] = useState<LoaderPhase>("checking");
  const [loaderText, setLoaderText] = useState("CHECKING FOR UPDATES...");
  const [activePage, setActivePage] = useState<Page>("freenet");
  const [appState, setAppState] = useState<AppState>({
    tg_proxy: {
      name: "tg-ws-proxy",
      installed: false,
      installedVersion: null,
      latestVersion: null,
      running: false,
    },
    zapret: {
      name: "zapret-discord-youtube",
      installed: false,
      installedVersion: null,
      latestVersion: null,
      running: false,
    },
  });

  useEffect(() => {
    checkForUpdate();
  }, []);

  const checkForUpdate = async () => {
    try {
      const info = await invoke<{
        current_version: string;
        latest_version: string;
        needs_update: boolean;
        download_url: string;
      }>("check_app_update");

      if (info.needs_update && info.download_url) {
        setLoaderText(`UPDATING v${info.current_version} → v${info.latest_version}...`);
        setLoaderPhase("updating");
        await invoke("apply_app_update", { downloadUrl: info.download_url });
        // App will restart via batch script — this line won't be reached
      }
    } catch (e) {
      console.warn("Update check failed:", e);
    }
    setLoaderPhase("ready");
  };

  const isAnyRunning = appState.tg_proxy.running || appState.zapret.running;

  if (loaderPhase !== "ready") {
    return (
      <div className="w-screen h-screen flex items-center justify-center overflow-hidden relative" style={{ backgroundColor: "#0a0118" }}>
        <div className="absolute inset-0 bg-mesh pointer-events-none" />
        <div className="absolute w-[900px] h-[900px] bg-primary/10 rounded-full blur-[160px] -top-80 -left-80 mix-blend-screen pointer-events-none opacity-50" />
        <div className="absolute w-[700px] h-[700px] bg-tertiary-container/10 rounded-full blur-[140px] -bottom-48 -right-48 mix-blend-screen pointer-events-none opacity-50" />
        <div className="flex flex-col items-center gap-6 z-10">
          <div className="relative">
            <div className={`w-20 h-20 rounded-full border-2 border-primary/30 border-t-primary ${loaderPhase === "updating" ? "animate-spin" : "animate-spin"}`} />
            <div className="absolute inset-0 flex items-center justify-center">
              <span className="material-symbols-outlined text-[36px] text-primary">
                {loaderPhase === "updating" ? "system_update" : "bolt"}
              </span>
            </div>
          </div>
          <div className="text-center">
            <h1 className="font-headline text-headline-sm text-on-surface tracking-[0.3em] font-bold uppercase mb-2">
              FREENET
            </h1>
            <p className={`font-body text-[12px] tracking-widest ${loaderPhase === "updating" ? "text-green-400/80 animate-pulse" : "text-on-surface-variant opacity-50"}`}>
              {loaderText}
            </p>
          </div>
        </div>
      </div>
    );
  }

  const renderPage = () => {
    switch (activePage) {
      case "downloads":
        return <DownloadsPage appState={appState} setAppState={setAppState} />;
      case "settings":
        return <SettingsPage />;
      default:
        return (
          <FreenetPage appState={appState} setAppState={setAppState} />
        );
    }
  };

  return (
    <div className="w-screen h-screen flex items-center justify-center overflow-hidden relative">
      <div className="w-[920px] h-[720px] rounded-[32px] bg-[#0a0118] flex flex-col relative z-10 overflow-hidden border border-white/10">
        <div className="absolute inset-0 bg-mesh pointer-events-none" />
        <div className="absolute w-[900px] h-[900px] bg-primary/10 rounded-full blur-[160px] -top-80 -left-80 mix-blend-screen pointer-events-none opacity-50" />
        <div className="absolute w-[700px] h-[700px] bg-tertiary-container/10 rounded-full blur-[140px] -bottom-48 -right-48 mix-blend-screen pointer-events-none opacity-50" />
        <div className="glass-shine" />

        <TitleBar />

        <NavBar activePage={activePage} onPageChange={setActivePage} />

        <main className="flex-1 flex flex-col items-center justify-center relative overflow-hidden">
          {renderPage()}
        </main>

        <StatusBar isRunning={isAnyRunning} />
      </div>
    </div>
  );
}

export default App;
