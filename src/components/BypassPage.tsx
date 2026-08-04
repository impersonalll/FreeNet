import { useState, useEffect, useRef, useCallback } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

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

export default function BypassPage() {
  const toast = useToast();
  const [services, setServices] = useState<BypassServiceInfo[]>([]);
  const [selectedBypass, setSelectedBypass] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  const load = useCallback(async () => {
    try {
      const [list, saved] = await Promise.all([
        invoke<BypassServiceInfo[]>("get_bypass_services"),
        invoke<string | null>("load_config_value", { key: "selected_bypass" }),
      ]);
      setServices(list);
      const effective = saved ?? "zapret";
      setSelectedBypass(effective);
      if (!saved) {
        invoke("save_config_value", { key: "selected_bypass", value: effective }).catch(() => {});
      }
    } catch (e) {
      console.error("Failed to load bypass services:", e);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load, reloadKey]);

  const refresh = () => setReloadKey((k) => k + 1);

  const selectBypass = async (key: string) => {
    setSelectedBypass(key);
    try {
      await invoke("save_config_value", { key: "selected_bypass", value: key });
    } catch (e) {
      console.error("Failed to save selection:", e);
    }
  };

  const handleDownload = async (key: string) => {
    setBusy(key);
    try {
      const msg = await invoke<string>("download_bypass", { key });
      toast.success("Installed", msg);
      if (["zapret", "goodbyedpi", "byedpi"].includes(key)) await selectBypass(key);
      refresh();
    } catch (e) {
      toast.error("Install failed", String(e));
    } finally {
      setBusy(null);
    }
  };

  // The currently running exclusive bypass, if any.
  const runningKey = services.find((s) => s.exclusive && s.running)?.key ?? null;
  // The user-selected bypass (falls back to whatever is running).
  const activeKey = runningKey ?? selectedBypass;

  return (
    <div className="flex-1 flex flex-col items-center relative w-full px-12 py-8 overflow-y-auto">
      <div className="absolute inset-0 bg-gradient-to-b from-outline/5 via-transparent to-transparent pointer-events-none" />

      <div className="w-full max-w-[640px] z-10">
        <div className="text-center mb-8">
          <h2 className="font-headline text-headline-sm text-on-surface tracking-[0.2em] font-bold uppercase mb-3">
            BYPASS
          </h2>
          <p className="font-body text-body-md text-on-surface-variant opacity-70">
            Install, select and run one bypass tool at a time.
          </p>
        </div>

        <div className="space-y-4">
          {services.map((s) => {
            const isExclusive = s.exclusive;
            const isActive = activeKey === s.key;
            // Another exclusive bypass is running, so this one cannot start.
            const blockedByRunning = isExclusive && runningKey !== null && runningKey !== s.key;
            const needsUpdate =
              s.installed && s.latest_version && s.installed_version !== s.latest_version;

            return (
              <div
                key={s.key}
                className={`glass-card rounded-2xl p-5 border bg-surface/30 liquid-blur transition-all duration-300 ${
                  s.running
                    ? "border-green-400/30"
                    : isActive
                    ? "border-primary/40"
                    : "border-white/10"
                }`}
              >
                <div className="flex items-center justify-between mb-1">
                  <div className="flex items-center gap-3">
                    <div
                      className={`w-3 h-3 rounded-full transition-all duration-500 ${
                        s.running
                          ? "bg-green-400 shadow-[0_0_12px_rgba(74,222,128,0.8)]"
                          : s.installed
                          ? "bg-primary/70 shadow-[0_0_10px_rgba(188,19,254,0.5)]"
                          : "bg-outline/40"
                      }`}
                    />
                    <h3 className="font-label text-[14px] text-on-surface tracking-[0.05em] font-bold uppercase">
                      {s.name}
                    </h3>
                  </div>
                  <div className="flex items-center gap-2">
                    {s.installed && s.installed_version && (
                      <span className="font-label text-[11px] text-outline/60 tracking-wider">
                        v{s.installed_version}
                      </span>
                    )}
                    {s.running && (
                      <span className="px-2 py-0.5 rounded-full bg-green-500/15 border border-green-400/30 text-green-400 text-[10px] font-bold tracking-wider">
                        RUNNING
                      </span>
                    )}
                    {needsUpdate && (
                      <span className="px-2 py-0.5 rounded-full bg-primary-container/30 text-primary text-[10px] font-bold tracking-wider">
                        UPDATE AVAILABLE
                      </span>
                    )}
                  </div>
                </div>

                <p className="font-body text-[11px] text-on-surface-variant opacity-60 mt-1 mb-4">
                  {s.description}
                </p>

                <button
                  onClick={() => handleDownload(s.key)}
                  disabled={busy === s.key}
                  className={`w-full py-2.5 rounded-xl font-label text-[12px] tracking-wider font-bold transition-all duration-300 active:scale-[0.98] disabled:opacity-50 ${
                    !s.installed
                      ? "bg-primary/20 hover:bg-primary/30 border border-primary/30 text-primary"
                      : needsUpdate
                      ? "bg-primary/15 hover:bg-primary/25 border border-primary/20 text-primary"
                      : "bg-surface-container-high/60 hover:bg-surface-container-high/80 border border-primary/15 text-on-surface-variant"
                  }`}
                >
                  {busy === s.key
                    ? "..."
                    : !s.installed
                    ? "INSTALL"
                    : needsUpdate
                    ? "UPDATE"
                    : "REINSTALL"}
                </button>

                {busy === s.key && (
                  <div className="mt-3 h-1 bg-surface-container-high/60 rounded-full overflow-hidden">
                    <div className="h-full bg-primary/60 rounded-full animate-pulse w-full" />
                  </div>
                )}

                {!s.installed && (
                  <p className="mt-3 font-body text-[11px] text-outline/50 text-center">
                    Not installed. Click INSTALL to get the latest version.
                  </p>
                )}

                {s.installed && (
                  <div className="mt-3 flex gap-2">
                    {s.running ? (
                      <button
                        disabled
                        className="flex-1 py-2 rounded-lg bg-green-500/10 border border-green-400/30 text-green-400 text-[11px] font-bold tracking-wider cursor-default"
                      >
                        RUNNING
                      </button>
                    ) : blockedByRunning ? (
                      <button
                        disabled
                        title="Only one bypass can run at a time"
                        className="flex-1 py-2 rounded-lg bg-surface-container-high/40 border border-primary/15 text-outline/40 text-[11px] font-bold tracking-wider cursor-not-allowed"
                      >
                        BLOCKED
                      </button>
                    ) : isExclusive && isActive ? (
                      <button
                        disabled
                        className="flex-1 py-2 rounded-lg bg-surface-container-high/40 border border-primary/15 text-outline/40 text-[11px] font-bold tracking-wider cursor-not-allowed"
                      >
                        SELECTED
                      </button>
                    ) : isExclusive ? (
                      <button
                        onClick={() => selectBypass(s.key)}
                        className="flex-1 py-2 rounded-lg bg-surface-container-high/60 hover:bg-surface-container-high/80 border border-primary/15 text-on-surface-variant text-[11px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97]"
                      >
                        SELECT
                      </button>
                    ) : null}
                  </div>
                )}

                {s.key === "zapret" && s.installed && <ZapretSettings />}
              </div>
            );
          })}
        </div>

        <div className="mt-4 p-3 rounded-lg bg-yellow-500/10 border border-yellow-500/20">
          <p className="font-body text-[11px] text-yellow-300/80 leading-relaxed">
            <span className="font-bold">NOTE:</span> zapret, GoodbyeDPI and ByeDPI all bypass DPI filtering (Discord, YouTube, Spotify...) and are mutually exclusive — only one can run. tg-ws-proxy runs alongside any of them.
          </p>
        </div>
      </div>
    </div>
  );
}

function ZapretSettings() {
  const [availableBatFiles, setAvailableBatFiles] = useState<string[]>([]);
  const [zapretBatFile, setZapretBatFile] = useState("general.bat");
  const [open, setOpen] = useState(false);

  useEffect(() => {
    loadBatFiles();
    loadSavedBatFile();
  }, []);

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

  const saveBatFile = async (file: string) => {
    setZapretBatFile(file);
    try {
      await invoke("save_config_value", { key: "zapret_bat_file", value: file });
    } catch (e) {
      console.error("Failed to save config:", e);
    }
  };

  return (
    <div className="mt-4 border-t border-white/5 pt-3">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1.5 text-[11px] text-primary/80 font-bold tracking-wider uppercase transition-colors hover:text-primary"
      >
        <span
          className={`material-symbols-outlined text-[15px] transition-transform duration-200 ${open ? "rotate-90" : ""}`}
        >
          chevron_right
        </span>
        Zapret settings
      </button>

      {open && (
        <div className="mt-3 space-y-3 animate-dropdown-in">
          <div>
            <h5 className="font-label text-[11px] text-on-surface-variant tracking-[0.1em] font-bold uppercase mb-1.5">
              Strategy (bat file)
            </h5>
            {availableBatFiles.length === 0 ? (
              <p className="text-[12px] text-outline/50 italic">No bat files found.</p>
            ) : (
              <BatDropdown value={zapretBatFile} options={availableBatFiles} onChange={saveBatFile} />
            )}
          </div>
          <div>
            <h5 className="font-label text-[11px] text-on-surface-variant tracking-[0.1em] font-bold uppercase mb-1.5">
              Version
            </h5>
            <ReleaseSelector />
          </div>
          <CustomDomains />
        </div>
      )}
    </div>
  );
}

function CustomDomains() {
  const [domains, setDomains] = useState<string[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);

  const load = async () => {
    try {
      const list = await invoke<string[]>("get_zapret_user_domains");
      setDomains(list);
    } catch (e) {
      console.error("Failed to load custom domains:", e);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const add = async () => {
    const domain = input.trim().toLowerCase();
    if (!domain) return;
    setBusy(true);
    try {
      const list = await invoke<string[]>("add_zapret_user_domain", { domain });
      setDomains(list);
      setInput("");
    } catch (e) {
      console.error("Failed to add domain:", e);
    } finally {
      setBusy(false);
    }
  };

  const remove = async (domain: string) => {
    setBusy(true);
    try {
      const list = await invoke<string[]>("remove_zapret_user_domain", { domain });
      setDomains(list);
    } catch (e) {
      console.error("Failed to remove domain:", e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="pt-1">
      <h5 className="font-label text-[11px] text-on-surface-variant tracking-[0.1em] font-bold uppercase mb-1.5">
        Custom domains
      </h5>
      <p className="font-body text-[10.5px] text-on-surface-variant/60 mb-2 leading-relaxed">
        Extra domains to bypass, saved to <span className="font-mono text-primary/80">list-general-user.txt</span>.
      </p>

      <div className="flex gap-2 mb-2">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") add();
          }}
          placeholder="example.com"
          className="flex-1 bg-surface-container-high/60 rounded-lg px-3 py-2 font-mono text-[12px] text-on-surface placeholder:text-outline/40 border border-primary/15 focus:border-primary/40 focus:outline-none focus:ring-1 focus:ring-primary/30 transition-all duration-200"
        />
        <button
          onClick={add}
          disabled={busy || !input.trim()}
          className="px-3 py-2 rounded-lg bg-primary/15 hover:bg-primary/25 border border-primary/20 text-primary text-[11px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97] shrink-0 disabled:opacity-40"
        >
          ADD
        </button>
      </div>

      {domains.length === 0 ? (
        <p className="text-[12px] text-outline/50 italic">No custom domains added.</p>
      ) : (
        <div className="space-y-1 max-h-[150px] overflow-y-auto pr-1">
          {domains.map((d) => (
            <div
              key={d}
              className="flex items-center justify-between gap-2 px-3 py-1.5 rounded-lg bg-surface-container-high/40 border border-primary/10"
            >
              <span className="font-mono text-[12px] text-on-surface truncate">{d}</span>
              <button
                onClick={() => remove(d)}
                disabled={busy}
                className="w-6 h-6 rounded-md flex items-center justify-center text-on-surface-variant/60 hover:text-red-400 hover:bg-red-500/10 transition-all duration-150 shrink-0 disabled:opacity-40"
              >
                <span className="material-symbols-outlined text-[14px]">close</span>
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ReleaseSelector() {
  const [releases, setReleases] = useState<{ tag: string; version: string; date: string }[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [installedVersion, setInstalledVersion] = useState<string | null>(null);

  useEffect(() => {
    loadReleases();
    loadInstalled();
  }, []);

  const loadReleases = async () => {
    setLoading(true);
    setError("");
    try {
      const list = await invoke<{ tag: string; version: string; date: string }[]>("list_releases", {
        service: "zapret-discord-youtube",
      });
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

  return (
    <div>
      {loading ? (
        <p className="text-[12px] text-outline/50 italic">Loading releases...</p>
      ) : error ? (
        <div className="text-center">
          <p className="text-[12px] text-red-400/80 mb-2">Failed to load releases</p>
          <button onClick={loadReleases} className="text-[11px] text-primary hover:underline">Retry</button>
        </div>
      ) : releases.length === 0 ? (
        <p className="text-[12px] text-outline/50 italic">No releases found.</p>
      ) : (
        <div className="space-y-1.5 max-h-[160px] overflow-y-auto pr-1">
          {releases.map((r) => (
            <div
              key={r.tag}
              className={`flex items-center justify-between px-3 py-2 rounded-lg border transition-all duration-200 ${
                installedVersion === r.version
                  ? "bg-green-500/10 border-green-500/20"
                  : "bg-surface-container-high/30 border-primary/10 hover:border-primary/30"
              }`}
            >
              <span className="font-label text-[12px] text-on-surface font-bold">v{r.version}</span>
              {installedVersion === r.version ? (
                <span className="px-1.5 py-0.5 rounded bg-green-500/20 text-green-400 text-[9px] font-bold tracking-wider">
                  INSTALLED
                </span>
              ) : (
                <span className="px-1.5 py-0.5 rounded bg-surface-container-high/60 text-outline/50 text-[9px] font-bold tracking-wider">
                  AVAILABLE
                </span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function BatDropdown({
  value,
  options,
  onChange,
}: {
  value: string;
  options: string[];
  onChange: (file: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [rect, setRect] = useState<{ top: number; left: number; width: number } | null>(null);
  const btnRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const update = () => {
      const el = btnRef.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      setRect({ top: r.bottom, left: r.left, width: r.width });
    };
    update();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    document.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
      document.removeEventListener("keydown", onKey, true);
    };
  }, [open]);

  return (
    <>
      <button
        ref={btnRef}
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-haspopup="listbox"
        aria-expanded={open}
        className={`w-full flex items-center justify-between gap-2 px-3 py-2.5 rounded-lg border font-body text-[13px] text-on-surface transition-all duration-200 cursor-pointer relative z-[60] ${
          open
            ? "bg-surface-container-high/80 border-primary/40 shadow-[0_0_20px_rgba(188,19,254,0.15)]"
            : "bg-surface-container-high/60 border-primary/15 hover:border-primary/35 hover:bg-surface-container-high/80"
        }`}
      >
        <span className="flex items-center gap-2 min-w-0">
          <span className="material-symbols-outlined text-[17px] text-primary/70 shrink-0">description</span>
          <span className="truncate">{value.replace(".bat", "")}</span>
        </span>
        <span
          className={`material-symbols-outlined text-[18px] text-on-surface-variant transition-transform duration-200 shrink-0 ${
            open ? "rotate-180" : ""
          }`}
        >
          expand_more
        </span>
      </button>

      {open &&
        createPortal(
          <div className="fixed inset-0 z-[200]">
            <div className="absolute inset-0" onClick={() => setOpen(false)} />
            {rect && (
              <div
                role="listbox"
                style={{ top: rect.top + 6, left: rect.left, width: rect.width }}
                className="absolute rounded-xl border border-white/10 bg-[#0e021d]/95 glass-modal shadow-[0_20px_50px_rgba(0,0,0,0.6)] overflow-hidden animate-dropdown-in"
              >
                <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent" />
                <div className="max-h-[220px] overflow-y-auto py-1">
                  {options.map((f) => {
                    const selected = f === value;
                    return (
                      <button
                        key={f}
                        role="option"
                        aria-selected={selected}
                        onClick={() => {
                          onChange(f);
                          setOpen(false);
                        }}
                        className={`w-full flex items-center justify-between gap-2 px-3 py-2 text-left font-body text-[12.5px] transition-colors duration-150 cursor-pointer ${
                          selected ? "text-primary bg-primary/10" : "text-on-surface hover:bg-white/5"
                        }`}
                      >
                        <span className="truncate">{f.replace(".bat", "")}</span>
                        {selected && (
                          <span className="material-symbols-outlined text-[16px] text-primary shrink-0">check</span>
                        )}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>,
          document.body
        )}
    </>
  );
}
