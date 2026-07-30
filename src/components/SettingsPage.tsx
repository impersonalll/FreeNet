import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ReleaseInfo {
  tag: string;
  version: string;
  date: string;
}

export default function SettingsPage() {
  const [zapretBatFile, setZapretBatFile] = useState("general.bat");
  const [availableBatFiles, setAvailableBatFiles] = useState<string[]>([]);
  const [dataDir, setDataDir] = useState("");
  const [downloadDir, setDownloadDir] = useState("");
  const [hostsBypass, setHostsBypass] = useState(false);
  const [hostsLoading, setHostsLoading] = useState(false);

  useEffect(() => {
    loadDataDir();
    loadDownloadDir();
    loadBatFiles();
    loadHostsStatus();
    loadSavedBatFile();
  }, []);

  const loadDataDir = async () => {
    try {
      const dir = await invoke<string>("get_data_dir_path");
      setDataDir(dir);
    } catch (e) {
      console.error("Failed to get data dir:", e);
    }
  };

  const loadDownloadDir = async () => {
    try {
      const dir = await invoke<string>("get_download_dir_path");
      setDownloadDir(dir);
    } catch (e) {
      console.error("Failed to get download dir:", e);
    }
  };

  const selectDownloadDir = async () => {
    try {
      const dir = await invoke<string | null>("select_download_dir");
      if (dir) setDownloadDir(dir);
    } catch (e) {
      console.error("Failed to select dir:", e);
    }
  };

  const loadBatFiles = async () => {
    try {
      const files = await invoke<string[]>("list_bat_files");
      setAvailableBatFiles(files);
    } catch (e) {
      console.error("Failed to load bat files:", e);
    }
  };

  const loadSavedBatFile = async () => {
    try {
      const saved = await invoke<string | null>("load_config_value", { key: "zapret_bat_file" });
      if (saved) setZapretBatFile(saved);
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  };

  const loadHostsStatus = async () => {
    try {
      const status = await invoke<boolean>("get_hosts_status");
      setHostsBypass(status);
    } catch (e) {
      console.error("Failed to check hosts status:", e);
    }
  };

  const saveBatFile = async (file: string) => {
    setZapretBatFile(file);
    try {
      await invoke("save_config_value", { key: "zapret_bat_file", value: file });
    } catch (e) {
      console.error("Failed to save config:", e);
    }
  };

  const toggleHostsBypass = async () => {
    setHostsLoading(true);
    try {
      const newState = !hostsBypass;
      await invoke("set_hosts_bypass", { enabled: newState });
      setHostsBypass(newState);
    } catch (e) {
      console.error("Failed to toggle hosts:", e);
      alert(`Error: ${e}`);
    } finally {
      setHostsLoading(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col items-center relative w-full px-12 py-8 overflow-y-auto">
      <div className="absolute inset-0 bg-gradient-to-b from-outline/5 via-transparent to-transparent pointer-events-none" />

      <div className="w-full max-w-[600px] z-10">
        <div className="text-center mb-10">
          <h2 className="font-headline text-headline-sm text-on-surface tracking-[0.2em] font-bold uppercase mb-3">
            SETTINGS
          </h2>
          <p className="font-body text-body-md text-on-surface-variant opacity-70">
            Configure application preferences and service parameters.
          </p>
        </div>

        <div className="space-y-8">
          {/* Bat file selector */}
          <SettingsSection title="zapret Strategy">
            <div className="glass-card rounded-xl p-4 border border-white/10 bg-surface/30 liquid-blur">
              <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold mb-1">
                Bat file
              </h4>
              <p className="font-body text-[12px] text-on-surface-variant opacity-60 mb-3">
                Which strategy zapret uses when launching
              </p>
              {availableBatFiles.length === 0 ? (
                <p className="text-[12px] text-outline/50 italic">
                  No bat files found. Download zapret first.
                </p>
              ) : (
                <select
                  value={zapretBatFile}
                  onChange={(e) => saveBatFile(e.target.value)}
                  className="w-full bg-surface-container-high/60 border border-white/8 rounded-lg px-3 py-2 font-body text-[13px] text-on-surface focus:outline-none focus:border-primary/40 transition-all duration-300"
                >
                  {availableBatFiles.map((f) => (
                    <option key={f} value={f}>
                      {f.replace(".bat", "")}
                    </option>
                  ))}
                </select>
              )}
            </div>
          </SettingsSection>

          {/* Release selector */}
          <SettingsSection title="zapret Version">
            <ReleaseSelector />
          </SettingsSection>

          {/* Hosts bypass */}
          <SettingsSection title="Other Sites Bypass">
            <div className="glass-card rounded-xl p-4 border border-white/10 bg-surface/30 liquid-blur">
              <div className="flex items-center justify-between mb-2">
                <div>
                  <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold">
                    Hosts file modification
                  </h4>
                  <p className="font-body text-[12px] text-on-surface-variant opacity-60 mt-0.5">
                    Add entries to Windows hosts file to bypass some blocks
                  </p>
                </div>
                <button
                  onClick={toggleHostsBypass}
                  disabled={hostsLoading}
                  className={`w-12 h-6 rounded-full transition-all duration-300 relative cursor-pointer shrink-0 ml-4 ${
                    hostsBypass
                      ? "bg-primary-container shadow-[0_0_12px_rgba(188,19,254,0.4)]"
                      : "bg-surface-container-high border border-white/10"
                  } ${hostsLoading ? "opacity-50" : ""}`}
                >
                  <div
                    className={`w-5 h-5 rounded-full bg-white absolute top-0.5 transition-all duration-300 ${
                      hostsBypass ? "left-[26px]" : "left-0.5"
                    }`}
                  />
                </button>
              </div>
              <div className="mt-3 p-3 rounded-lg bg-yellow-500/10 border border-yellow-500/20">
                <p className="font-body text-[11px] text-yellow-300/80 leading-relaxed">
                  <span className="font-bold">WARNING:</span> This modifies the Windows hosts file. Requires admin rights. May not work for all sites — only effective for DNS-level blocks. IP-level blocks need a VPN/proxy. Domains: SoundCloud, Gemini, ChatGPT, OpenAI, Telegram.
                </p>
              </div>
            </div>
          </SettingsSection>

          {/* Paths */}
          <SettingsSection title="Paths">
            <div className="glass-card rounded-xl p-4 border border-white/10 bg-surface/30 liquid-blur">
              <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold mb-1">
                Download directory
              </h4>
              <p className="font-body text-[12px] text-on-surface-variant opacity-60 mb-2">
                Where zapret and tg-ws-proxy files are stored
              </p>
              <div className="flex gap-2">
                <code className="flex-1 bg-surface-container-high/60 border border-white/8 rounded-lg px-3 py-2 font-mono text-[12px] text-primary/80 break-all truncate">
                  {downloadDir || "Loading..."}
                </code>
                <button
                  onClick={selectDownloadDir}
                  className="px-4 py-2 rounded-lg bg-primary/15 hover:bg-primary/25 border border-primary/20 text-primary text-[11px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97] shrink-0"
                >
                  CHANGE
                </button>
              </div>
            </div>

            <div className="glass-card rounded-xl p-4 border border-white/10 bg-surface/30 liquid-blur">
              <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold mb-1">
                App data directory
              </h4>
              <p className="font-body text-[12px] text-on-surface-variant opacity-60 mb-2">
                Internal application data
              </p>
              <code className="block bg-surface-container-high/60 border border-white/8 rounded-lg px-3 py-2 font-mono text-[12px] text-outline/50 break-all">
                {dataDir || "Loading..."}
              </code>
            </div>
          </SettingsSection>
        </div>
      </div>
    </div>
  );
}

function ReleaseSelector() {
  const [releases, setReleases] = useState<ReleaseInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [selectedVersion, setSelectedVersion] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadMsg, setDownloadMsg] = useState("");
  const [installedVersion, setInstalledVersion] = useState<string | null>(null);

  useEffect(() => {
    loadReleases();
    loadInstalled();
  }, []);

  const loadReleases = async () => {
    setLoading(true);
    setError("");
    try {
      const list = await invoke<ReleaseInfo[]>("list_releases", { service: "zapret-discord-youtube" });
      setReleases(list);
    } catch (e) {
      console.error("Failed to load releases:", e);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const loadInstalled = async () => {
    try {
      const v = await invoke<string | null>("get_installed_version", { service: "zapret-discord-youtube" });
      setInstalledVersion(v);
    } catch (e) {
      console.error(e);
    }
  };

  const handleDownload = async (version: string) => {
    setDownloading(true);
    setDownloadMsg(`Downloading v${version}...`);
    try {
      const result = await invoke<string>("download_release", {
        service: "zapret-discord-youtube",
        version: version,
      });
      setDownloadMsg(result);
      setInstalledVersion(version);
      setTimeout(() => setDownloadMsg(""), 3000);
    } catch (e) {
      setDownloadMsg(`Error: ${e}`);
    } finally {
      setDownloading(false);
    }
  };

  return (
    <div className="glass-card rounded-xl p-4 border border-white/10 bg-surface/30 liquid-blur">
      <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold mb-1">
        Select version
      </h4>
      <p className="font-body text-[12px] text-on-surface-variant opacity-60 mb-3">
        Choose a specific release to install
      </p>

      {loading ? (
        <p className="text-[12px] text-outline/50 italic">Loading releases...</p>
      ) : error ? (
        <div className="text-center">
          <p className="text-[12px] text-red-400/80 mb-2">Failed to load releases</p>
          <p className="text-[10px] text-outline/40 break-all">{error}</p>
          <button onClick={loadReleases} className="mt-2 text-[11px] text-primary hover:underline">Retry</button>
        </div>
      ) : releases.length === 0 ? (
        <p className="text-[12px] text-outline/50 italic">No releases found.</p>
      ) : (
        <div className="space-y-1.5 max-h-[200px] overflow-y-auto pr-1">
          {releases.map((r) => (
            <div
              key={r.tag}
              className={`flex items-center justify-between px-3 py-2 rounded-lg border transition-all duration-200 ${
                installedVersion === r.version
                  ? "bg-green-500/10 border-green-500/20"
                  : selectedVersion === r.version
                  ? "bg-primary/10 border-primary/30"
                  : "bg-surface-container-high/30 border-white/5 hover:border-white/15"
              }`}
            >
              <div className="flex items-center gap-2">
                <span className="font-label text-[12px] text-on-surface font-bold">
                  v{r.version}
                </span>
                {installedVersion === r.version && (
                  <span className="px-1.5 py-0.5 rounded bg-green-500/20 text-green-400 text-[9px] font-bold tracking-wider">
                    INSTALLED
                  </span>
                )}
              </div>
              <button
                onClick={() => handleDownload(r.version)}
                disabled={downloading}
                className="px-3 py-1 rounded-lg bg-primary/15 hover:bg-primary/25 border border-primary/20 text-primary text-[10px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97] disabled:opacity-40"
              >
                {downloading && selectedVersion === r.version ? "..." : "INSTALL"}
              </button>
            </div>
          ))}
        </div>
      )}

      {downloadMsg && (
        <p className="mt-3 font-body text-[11px] text-primary/80 text-center">{downloadMsg}</p>
      )}
    </div>
  );
}

function SettingsSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <h3 className="font-label text-label-md text-primary/80 tracking-[0.15em] font-bold uppercase mb-3">
        {title}
      </h3>
      <div className="space-y-2">{children}</div>
    </div>
  );
}
