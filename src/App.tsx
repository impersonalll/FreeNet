import { useState } from "react";
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

function App() {
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

  const isAnyRunning = appState.tg_proxy.running || appState.zapret.running;

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
