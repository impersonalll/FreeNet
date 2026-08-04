import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

interface HostsProvider {
  key: string;
  name: string;
  description: string;
  url: string | null;
  custom: boolean;
}

export default function SettingsPage() {
  const toast = useToast();
  const [dataDir, setDataDir] = useState("");
  const [downloadDir, setDownloadDir] = useState("");
  const [hostsBypass, setHostsBypass] = useState(false);
  const [hostsLoading, setHostsLoading] = useState(false);
  const [hostsProviders, setHostsProviders] = useState<HostsProvider[]>([]);
  const [selectedProviders, setSelectedProviders] = useState<string[]>([]);
  const [customHostsUrl, setCustomHostsUrl] = useState("");

  useEffect(() => {
    loadDataDir();
    loadDownloadDir();
    loadHostsStatus();
    loadHostsProviders();
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

  const loadHostsStatus = async () => {
    try {
      const status = await invoke<boolean>("get_hosts_status");
      setHostsBypass(status);
    } catch (e) {
      console.error("Failed to check hosts status:", e);
    }
  };

  const toggleHostsBypass = async () => {
    setHostsLoading(true);
    try {
      const newState = !hostsBypass;
      if (newState && selectedProviders.length === 0) {
        toast.warning("No providers selected", "Select at least one hosts provider first.");
        setHostsLoading(false);
        return;
      }
      await invoke("set_hosts_bypass", { enabled: newState });
      setHostsBypass(newState);
      if (newState) {
        toast.success("Hosts bypass enabled", `Providers: ${selectedProviders.join(", ")}`);
      } else {
        toast.info("Hosts bypass disabled");
      }
    } catch (e) {
      console.error("Failed to toggle hosts:", e);
      toast.error("Failed to toggle hosts", String(e));
    } finally {
      setHostsLoading(false);
    }
  };

  const loadHostsProviders = async () => {
    try {
      const providers = await invoke<HostsProvider[]>("get_hosts_providers");
      setHostsProviders(providers);
      const saved = await invoke<string[]>("get_selected_hosts_providers");
      if (saved.length > 0) setSelectedProviders(saved);
      const savedUrl = await invoke<string | null>("load_config_value", { key: "custom_hosts_url" });
      if (savedUrl) setCustomHostsUrl(savedUrl);
    } catch (e) {
      console.error("Failed to load hosts providers:", e);
    }
  };

  const toggleHostsProvider = async (key: string) => {
    const next = selectedProviders.includes(key)
      ? selectedProviders.filter((p) => p !== key)
      : [...selectedProviders, key];
    setSelectedProviders(next);
    try {
      await invoke("set_selected_hosts_providers", { providers: next });
    } catch (e) {
      console.error("Failed to save hosts providers:", e);
    }
  };

  const saveCustomHostsUrl = async () => {
    try {
      await invoke("save_config_value", { key: "custom_hosts_url", value: customHostsUrl });
      toast.success("Custom hosts URL saved");
    } catch (e) {
      toast.error("Failed to save URL", String(e));
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
                      : "bg-surface-container-high border border-primary/15"
                  } ${hostsLoading ? "opacity-50" : ""}`}
                >
                  <div
                    className={`w-5 h-5 rounded-full bg-white absolute top-0.5 transition-all duration-300 ${
                      hostsBypass ? "left-[26px]" : "left-0.5"
                    }`}
                  />
                </button>
              </div>

              <div className="mt-4">
                <h5 className="font-label text-[11px] text-on-surface-variant tracking-[0.1em] font-bold uppercase mb-2">
                  Hosts providers (select one or more)
                </h5>
                <div className="space-y-1.5">
                  {hostsProviders.map((p) => {
                    const selected = selectedProviders.includes(p.key);
                    return (
                      <button
                        key={p.key}
                        type="button"
                        onClick={() => toggleHostsProvider(p.key)}
                        aria-pressed={selected}
                        className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg border font-body text-[13px] text-on-surface transition-all duration-200 cursor-pointer text-left ${
                          selected
                            ? "bg-primary/10 border-primary/40"
                            : "bg-surface-container-high/40 border-primary/15 hover:border-primary/30"
                        }`}
                      >
                        <span
                          className={`w-5 h-5 rounded-md border flex items-center justify-center shrink-0 transition-all duration-200 ${
                            selected
                              ? "bg-primary border-primary"
                              : "bg-surface-container-high border-primary/25"
                          }`}
                        >
                          {selected && (
                            <span className="material-symbols-outlined text-[14px] text-white">check</span>
                          )}
                        </span>
                        <span className="min-w-0">
                          <span className="block font-label text-[12.5px] font-bold">{p.name}</span>
                          <span className="block text-[10.5px] text-on-surface-variant/60 truncate">
                            {p.description}
                          </span>
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>

              {selectedProviders.includes("custom") && (
                <div className="mt-3">
                  <h5 className="font-label text-[11px] text-on-surface-variant tracking-[0.1em] font-bold uppercase mb-2">
                    Custom hosts file URL
                  </h5>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={customHostsUrl}
                      onChange={(e) => setCustomHostsUrl(e.target.value)}
                      placeholder="https://example.com/hosts.txt"
                      className="flex-1 bg-surface-container-high/60 rounded-lg px-3 py-2 font-mono text-[12px] text-on-surface placeholder:text-outline/40 border border-primary/15 focus:border-primary/40 focus:outline-none focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                    />
                    <button
                      onClick={saveCustomHostsUrl}
                      className="px-3 py-2 rounded-lg bg-primary/15 hover:bg-primary/25 border border-primary/20 text-primary text-[11px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97] shrink-0"
                    >
                      SAVE
                    </button>
                  </div>
                </div>
              )}

              <div className="mt-3 p-3 rounded-lg bg-yellow-500/10 border border-yellow-500/20">
                <p className="font-body text-[11px] text-yellow-300/80 leading-relaxed">
                  <span className="font-bold">WARNING:</span> This modifies the Windows hosts file. Requires admin rights. Blocks advertising, trackers and bypasses DNS blocks. Remote lists are fetched and parsed when you enable the toggle.
                </p>
              </div>
            </div>
          </SettingsSection>

          <SettingsSection title="Paths">
            <div className="glass-card rounded-xl p-4 border border-white/10 bg-surface/30 liquid-blur">
              <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold mb-1">
                Download directory
              </h4>
              <p className="font-body text-[12px] text-on-surface-variant opacity-60 mb-2">
                Where zapret and tg-ws-proxy files are stored
              </p>
              <div className="flex gap-2">
                <code className="flex-1 bg-surface-container-high/60 border border-primary/15 rounded-lg px-3 py-2 font-mono text-[12px] text-primary/80 break-all truncate">
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
              <code className="block bg-surface-container-high/60 border border-primary/15 rounded-lg px-3 py-2 font-mono text-[12px] text-outline/50 break-all">
                {dataDir || "Loading..."}
              </code>
            </div>
          </SettingsSection>
        </div>
      </div>
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

