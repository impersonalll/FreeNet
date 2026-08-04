import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface HotkeyConfig {
  play_pause: string;
  next_track: string;
  prev_track: string;
}

const DEFAULT_HOTKEYS: HotkeyConfig = {
  play_pause: "MediaPlayPause",
  next_track: "MediaNextTrack",
  prev_track: "MediaPrevTrack",
};

const KEY_NAMES: Record<string, string> = {
  MediaPlayPause: "⏯ Play/Pause",
  MediaNextTrack: "⏭ Next Track",
  MediaPrevTrack: "⏮ Prev Track",
};

function formatCombo(combo: string): string {
  return KEY_NAMES[combo] || combo;
}

// Physical-key names (e.code) are layout-independent, so a bind captured in
// one layout works in any other (Russian/English). e.key would give the
// character instead ("а" in Russian), which the backend cannot parse.
const CODE_MAP: Record<string, string> = {
  Space: "Space",
  Backspace: "Backspace",
  Tab: "Tab",
  Enter: "Enter",
  Escape: "Escape",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Insert: "Insert",
  Delete: "Delete",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
};

function codeToKey(code: string): string {
  if (CODE_MAP[code]) return CODE_MAP[code];
  if (code.startsWith("Key")) return code.slice(3); // KeyF -> F
  if (code.startsWith("Digit")) return code.slice(5); // Digit1 -> 1
  if (/^F\d{1,2}$/.test(code)) return code; // F1-F24
  return code;
}

export default function PluginsPage() {
  const [hotkeys, setHotkeys] = useState<HotkeyConfig>(DEFAULT_HOTKEYS);
  const [registered, setRegistered] = useState(false);
  const [showModal, setShowModal] = useState(false);
  const [mediaKeysEnabled, setMediaKeysEnabled] = useState(true);
  const [clipboardEnabled, setClipboardEnabled] = useState(false);
  const [autoAcceptEnabled, setAutoAcceptEnabled] = useState(false);
  const [autoAcceptGames, setAutoAcceptGames] = useState<{ key: string; name: string }[]>([]);
  const [selectedAutoAcceptGames, setSelectedAutoAcceptGames] = useState<string[]>([]);
  const [showClipboard, setShowClipboard] = useState(false);
  const [clipboardHistory, setClipboardHistory] = useState<string[]>([]);
  const [copiedItem, setCopiedItem] = useState<number | null>(null);

  useEffect(() => {
    loadHotkeys();
    loadMediaKeysEnabled();
    loadClipboardEnabled();
    loadAutoAcceptEnabled();
    loadAutoAcceptGames();
  }, []);

  const loadMediaKeysEnabled = async () => {
    try {
      const saved = await invoke<string | null>("load_config_value", { key: "media_keys_enabled" });
      if (saved !== null) setMediaKeysEnabled(saved === "true");
    } catch (e) {
      console.warn("Failed to load plugin state:", e);
    }
  };

  const loadClipboardEnabled = async () => {
    try {
      const saved = await invoke<string | null>("load_config_value", { key: "clipboard_enabled" });
      if (saved !== null) setClipboardEnabled(saved === "true");
    } catch (e) {
      console.warn("Failed to load plugin state:", e);
    }
  };

  const loadAutoAcceptEnabled = async () => {
    try {
      const saved = await invoke<string | null>("load_config_value", { key: "auto_accept_enabled" });
      if (saved !== null) setAutoAcceptEnabled(saved === "true");
    } catch (e) {
      console.warn("Failed to load plugin state:", e);
    }
  };

  const loadAutoAcceptGames = async () => {
    try {
      const games = await invoke<{ key: string; name: string }[]>("get_auto_accept_games");
      setAutoAcceptGames(games);
      const saved = await invoke<string[]>("get_selected_auto_accept_games");
      if (saved.length > 0) setSelectedAutoAcceptGames(saved);
    } catch (e) {
      console.warn("Failed to load auto-accept games:", e);
    }
  };

  const toggleAutoAcceptGame = async (key: string) => {
    const next = selectedAutoAcceptGames.includes(key)
      ? selectedAutoAcceptGames.filter((g) => g !== key)
      : [...selectedAutoAcceptGames, key];
    setSelectedAutoAcceptGames(next);
    try {
      await invoke("set_selected_auto_accept_games", { games: next });
    } catch (e) {
      console.error("Failed to save auto-accept games:", e);
    }
  };

  const toggleMediaKeys = async () => {
    const next = !mediaKeysEnabled;
    setMediaKeysEnabled(next);
    try {
      await invoke("set_media_keys_enabled", { enabled: next });
    } catch (e) {
      console.error("Failed to toggle plugin:", e);
      setMediaKeysEnabled(!next);
    }
  };

  const toggleClipboard = async () => {
    const next = !clipboardEnabled;
    setClipboardEnabled(next);
    try {
      await invoke("set_clipboard_enabled", { enabled: next });
    } catch (e) {
      console.error("Failed to toggle plugin:", e);
      setClipboardEnabled(!next);
    }
  };

  const toggleAutoAccept = async () => {
    const next = !autoAcceptEnabled;
    setAutoAcceptEnabled(next);
    try {
      await invoke("set_auto_accept_enabled", { enabled: next });
    } catch (e) {
      console.error("Failed to toggle plugin:", e);
      setAutoAcceptEnabled(!next);
    }
  };

  const openClipboard = async () => {
    try {
      const history = await invoke<string[]>("get_clipboard_history");
      setClipboardHistory(history);
      setShowClipboard(true);
    } catch (e) {
      console.error("Failed to load clipboard history:", e);
    }
  };

  const copyClipboardItem = async (text: string, index: number) => {
    try {
      await invoke("copy_clipboard_item", { text });
      setCopiedItem(index);
      setTimeout(() => setCopiedItem(null), 1500);
    } catch (e) {
      console.error("Failed to copy item:", e);
    }
  };

  const loadHotkeys = async () => {
    try {
      const saved = await invoke<HotkeyConfig | null>("load_hotkeys");
      if (saved) setHotkeys(saved);
    } catch (e) {
      console.warn("Failed to load hotkeys:", e);
    }
  };

  const updateHotkey = async (key: keyof HotkeyConfig, value: string) => {
    const newHotkeys = { ...hotkeys, [key]: value };
    setHotkeys(newHotkeys);
    try {
      await invoke("save_hotkeys", { hotkeys: newHotkeys });
      await invoke("register_music_hotkeys", { hotkeys: newHotkeys });
      setRegistered(true);
      setTimeout(() => setRegistered(false), 2000);
    } catch (e) {
      console.error("Failed to save hotkeys:", e);
    }
  };

  return (
    <div className="flex flex-1 min-h-0 w-full relative">
      {/* Main plugins content */}
      <div className="flex-1 flex flex-col items-center relative px-12 py-8 overflow-y-auto min-w-0">
        <div className="absolute inset-0 bg-gradient-to-b from-outline/5 via-transparent to-transparent pointer-events-none" />

        <div className="w-full max-w-[600px] z-10">
          <div className="text-center mb-10">
            <h2 className="font-headline text-headline-sm text-on-surface tracking-[0.2em] font-bold uppercase mb-3">
              PLUGINS
            </h2>
            <p className="font-body text-body-md text-on-surface-variant opacity-70">
              Enhance your experience with powerful add-ons.
            </p>
          </div>

          <div className="space-y-8">
            <div>
              <h3 className="font-label text-label-md text-primary/80 tracking-[0.15em] font-bold uppercase mb-3">
                Music Control
              </h3>
              <div className="glass-card rounded-xl p-5 border border-white/10 bg-surface/30 liquid-blur">
                <div className="flex items-center gap-3 mb-5">
                  <span className="material-symbols-outlined text-[24px] text-primary">music_note</span>
                  <div className="flex-1">
                    <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold">
                      Global Media Keys
                    </h4>
                    <p className="font-body text-[11px] text-on-surface-variant opacity-60">
                      Control music from any window with global hotkeys
                    </p>
                  </div>
                  <button
                    onClick={toggleMediaKeys}
                    aria-pressed={mediaKeysEnabled}
                    className={`relative w-11 h-6 rounded-full transition-all duration-300 shrink-0 cursor-pointer ${
                      mediaKeysEnabled
                        ? "bg-primary shadow-[0_0_12px_rgba(188,19,254,0.5)]"
                        : "bg-surface-container-high border border-primary/15"
                    }`}
                  >
                    <div
                      className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-all duration-300 ${
                        mediaKeysEnabled ? "left-[22px]" : "left-0.5"
                      }`}
                    />
                  </button>
                  <button
                    onClick={() => setShowModal(true)}
                    disabled={!mediaKeysEnabled}
                    className="px-3 py-1.5 rounded-lg text-[11px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97] border bg-surface-container-high/60 border-primary/15 text-on-surface-variant hover:text-on-surface hover:border-primary/30 disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    <span className="material-symbols-outlined text-[14px] align-middle mr-1">keyboard</span>
                    KEYBINDS
                  </button>
                </div>

                <div className={`flex items-center justify-between px-3 py-2 rounded-lg bg-surface-container-high/40 border border-primary/10 transition-opacity duration-300 ${mediaKeysEnabled ? "" : "opacity-40"}`}>
                  <span className="font-label text-[12px] text-on-surface font-bold">Current binds</span>
                  <span className="font-body text-[11px] text-on-surface-variant/60">
                    {formatCombo(hotkeys.play_pause)} / {formatCombo(hotkeys.next_track)} / {formatCombo(hotkeys.prev_track)}
                  </span>
                </div>
              </div>
            </div>

            <div>
              <h3 className="font-label text-label-md text-primary/80 tracking-[0.15em] font-bold uppercase mb-3">
                Productivity
              </h3>
              <div className="glass-card rounded-xl p-5 border border-white/10 bg-surface/30 liquid-blur">
                <div className="flex items-center gap-3 mb-5">
                  <span className="material-symbols-outlined text-[24px] text-primary">content_paste</span>
                  <div className="flex-1">
                    <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold">
                      Clipboard Manager
                    </h4>
                    <p className="font-body text-[11px] text-on-surface-variant opacity-60">
                      Save copied text to a custom buffer, then paste it with a click
                    </p>
                  </div>
                  <button
                    onClick={toggleClipboard}
                    aria-pressed={clipboardEnabled}
                    className={`relative w-11 h-6 rounded-full transition-all duration-300 shrink-0 cursor-pointer ${
                      clipboardEnabled
                        ? "bg-primary shadow-[0_0_12px_rgba(188,19,254,0.5)]"
                        : "bg-surface-container-high border border-primary/15"
                    }`}
                  >
                    <div
                      className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-all duration-300 ${
                        clipboardEnabled ? "left-[22px]" : "left-0.5"
                      }`}
                    />
                  </button>
                  <button
                    onClick={openClipboard}
                    disabled={!clipboardEnabled}
                    className="px-3 py-1.5 rounded-lg text-[11px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97] border bg-surface-container-high/60 border-primary/15 text-on-surface-variant hover:text-on-surface hover:border-primary/30 disabled:opacity-40 disabled:cursor-not-allowed"
                  >
                    <span className="material-symbols-outlined text-[14px] align-middle mr-1">history</span>
                    BUFFER
                  </button>
                </div>
              </div>
            </div>

            <div>
              <h3 className="font-label text-label-md text-primary/80 tracking-[0.15em] font-bold uppercase mb-3">
                Gaming
              </h3>
              <div className="glass-card rounded-xl p-5 border border-white/10 bg-surface/30 liquid-blur">
                <div className="flex items-center gap-3 mb-5">
                  <span className="material-symbols-outlined text-[24px] text-primary">sports_esports</span>
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold">
                        Auto Accept Match
                      </h4>
                      <span className="px-1.5 py-0.5 rounded bg-amber-500/15 border border-amber-400/30 font-label text-[9px] font-bold tracking-[0.12em] text-amber-300">
                        IN DEV
                      </span>
                    </div>
                    <p className="font-body text-[11px] text-on-surface-variant opacity-60">
                      Automatically clicks ACCEPT when a match is found in your selected games
                    </p>
                  </div>
                  <button
                    onClick={toggleAutoAccept}
                    disabled
                    aria-pressed={false}
                    title="Not available yet — currently in development"
                    className={`relative w-11 h-6 rounded-full transition-all duration-300 shrink-0 cursor-not-allowed bg-surface-container-high border border-primary/15 opacity-40`}
                  >
                    <div
                      className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-all duration-300 left-0.5`}
                    />
                  </button>
                </div>

                <div className="mt-4">
                  <h5 className="font-label text-[11px] text-on-surface-variant tracking-[0.1em] font-bold uppercase mb-2">
                    Games (select one or more)
                  </h5>
                  <div className="space-y-1.5">
                    {autoAcceptGames.map((g) => {
                      const selected = selectedAutoAcceptGames.includes(g.key);
                      return (
                        <button
                          key={g.key}
                          type="button"
                          onClick={() => toggleAutoAcceptGame(g.key)}
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
                          <span className="font-label text-[12.5px] font-bold">{g.name}</span>
                        </button>
                      );
                    })}
                  </div>
                </div>

                <div className="mt-3 flex items-center gap-2 px-3 py-2 rounded-lg bg-surface-container-high/40 border border-primary/10 opacity-40">
                  <span className="material-symbols-outlined text-[14px] text-primary/70">construction</span>
                  <span className="font-body text-[11px] text-on-surface-variant/70">
                    Under development — cannot be enabled yet. Clicks are simulated directly in the game window without moving your cursor.
                  </span>
                </div>
              </div>
            </div>

            <div>
              <h3 className="font-label text-label-md text-primary/80 tracking-[0.15em] font-bold uppercase mb-3">
                Coming Soon
              </h3>
              <div className="glass-card rounded-xl p-5 border border-white/10 bg-surface/30 liquid-blur opacity-50">
                <div className="flex items-center gap-3">
                  <span className="material-symbols-outlined text-[24px] text-outline/40">construction</span>
                  <div>
                    <h4 className="font-label text-[13px] text-on-surface tracking-[0.03em] font-bold">
                      More plugins coming soon
                    </h4>
                    <p className="font-body text-[11px] text-on-surface-variant opacity-60">
                      Custom DPI rules, network monitor, auto-start, and more
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Keybind modal */}
      {showModal && (
        <div
          className="absolute inset-0 z-30 flex items-center justify-center"
          onClick={() => setShowModal(false)}
        >
          <div className="absolute inset-0 bg-black/50 backdrop-blur-sm animate-fade-in" />
          <div
            className="relative w-[440px] max-h-[85%] overflow-y-auto rounded-[24px] border border-white/10 bg-[#0e021d]/95 glass-modal shadow-[0_20px_60px_rgba(0,0,0,0.6)] animate-modal-in"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent" />

            <div className="flex items-center justify-between px-6 py-4 border-b border-white/10">
              <div className="flex items-center">
                <div className="w-9 h-9 rounded-xl bg-primary/15 border border-primary/25 flex items-center justify-center mr-3">
                  <span className="material-symbols-outlined text-[20px] text-primary">keyboard</span>
                </div>
                <div>
                  <h2 className="font-headline text-[14px] text-on-surface tracking-[0.15em] font-bold uppercase">
                    Keybinds
                  </h2>
                  <p className="font-body text-[10px] text-on-surface-variant opacity-60">
                    Global music control shortcuts
                  </p>
                </div>
              </div>
              <button
                onClick={() => setShowModal(false)}
                className="w-8 h-8 rounded-full flex items-center justify-center text-on-surface-variant hover:text-on-surface hover:bg-white/10 transition-all duration-200"
              >
                <span className="material-symbols-outlined text-[18px]">close</span>
              </button>
            </div>

            <div className="px-6 py-5">
              <div className="space-y-4">
                <KeybindCapture
                  label="Play / Pause"
                  icon="play_pause"
                  value={hotkeys.play_pause}
                  onChange={(v) => updateHotkey("play_pause", v)}
                />
                <KeybindCapture
                  label="Next Track"
                  icon="skip_next"
                  value={hotkeys.next_track}
                  onChange={(v) => updateHotkey("next_track", v)}
                />
                <KeybindCapture
                  label="Previous Track"
                  icon="skip_previous"
                  value={hotkeys.prev_track}
                  onChange={(v) => updateHotkey("prev_track", v)}
                />

                {registered && (
                  <p className="text-center text-[11px] text-green-400/80 animate-pulse">
                    Hotkeys registered!
                  </p>
                )}

                <div className="p-3 rounded-xl bg-primary/5 border border-primary/10">
                  <p className="font-body text-[10px] text-on-surface-variant/60 leading-relaxed">
                    Click "Capture" then press your desired key combination. Media keys work globally. Custom combos (Ctrl+F5 etc.) are intercepted before apps receive them.
                  </p>
                </div>

                <button
                  onClick={() => setShowModal(false)}
                  className="w-full py-2.5 rounded-xl bg-primary/20 hover:bg-primary/30 border border-primary/30 text-primary text-[11px] font-bold tracking-widest uppercase transition-all duration-200 active:scale-[0.98]"
                >
                  Done
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Clipboard buffer modal */}
      {showClipboard && (
        <div
          className="absolute inset-0 z-30 flex items-center justify-center"
          onClick={() => setShowClipboard(false)}
        >
          <div className="absolute inset-0 bg-black/50 backdrop-blur-sm animate-fade-in" />
          <div
            className="relative w-[480px] max-h-[85%] flex flex-col rounded-[24px] border border-white/10 bg-[#0e021d]/95 glass-modal shadow-[0_20px_60px_rgba(0,0,0,0.6)] animate-modal-in"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent" />

            <div className="flex items-center justify-between px-6 py-4 border-b border-white/10">
              <div className="flex items-center">
                <div className="w-9 h-9 rounded-xl bg-primary/15 border border-primary/25 flex items-center justify-center mr-3">
                  <span className="material-symbols-outlined text-[20px] text-primary">content_paste</span>
                </div>
                <div>
                  <h2 className="font-headline text-[14px] text-on-surface tracking-[0.15em] font-bold uppercase">
                    Clipboard Buffer
                  </h2>
                  <p className="font-body text-[10px] text-on-surface-variant opacity-60">
                    Click an entry to copy it back
                  </p>
                </div>
              </div>
              <button
                onClick={() => setShowClipboard(false)}
                className="w-8 h-8 rounded-full flex items-center justify-center text-on-surface-variant hover:text-on-surface hover:bg-white/10 transition-all duration-200"
              >
                <span className="material-symbols-outlined text-[18px]">close</span>
              </button>
            </div>

            <div className="flex-1 overflow-y-auto px-6 py-5 space-y-2">
              {clipboardHistory.length === 0 ? (
                <div className="text-center py-10">
                  <span className="material-symbols-outlined text-[40px] text-outline/30 block mb-3">content_paste_off</span>
                  <p className="font-body text-[12px] text-on-surface-variant opacity-60">
                    Buffer is empty. Copy some text anywhere — it will appear here.
                  </p>
                </div>
              ) : (
                clipboardHistory.map((text, i) => (
                  <button
                    key={i}
                    onClick={() => copyClipboardItem(text, i)}
                    className="w-full text-left px-3.5 py-3 rounded-xl bg-surface-container-high/40 border border-primary/10 hover:border-primary/40 hover:bg-primary/10 transition-all duration-200 group"
                  >
                    <p className="font-body text-[12px] text-on-surface break-words line-clamp-3 leading-relaxed">
                      {text}
                    </p>
                    <p className="mt-1.5 text-[10px] tracking-wider font-bold uppercase text-on-surface-variant/50 group-hover:text-primary/70 flex items-center gap-1">
                      {copiedItem === i ? (
                        <>
                          <span className="material-symbols-outlined text-[13px] text-green-400">check</span>
                          <span className="text-green-400">Copied!</span>
                        </>
                      ) : (
                        <>
                          <span className="material-symbols-outlined text-[13px]">content_copy</span>
                          Copy
                        </>
                      )}
                    </p>
                  </button>
                ))
              )}
            </div>

            <div className="px-6 py-4 border-t border-white/10 flex items-center justify-between">
              <button
                onClick={async () => {
                  try {
                    await invoke("clear_clipboard_history");
                    setClipboardHistory([]);
                  } catch (e) {
                    console.error("Failed to clear history:", e);
                  }
                }}
                disabled={clipboardHistory.length === 0}
                className="px-3 py-1.5 rounded-lg text-[11px] font-bold tracking-wider border bg-surface-container-high/40 border-primary/15 text-red-400/80 hover:text-red-400 hover:border-red-400/30 transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed"
              >
                <span className="material-symbols-outlined text-[14px] align-middle mr-1">delete</span>
                CLEAR
              </button>
              <button
                onClick={() => setShowClipboard(false)}
                className="px-4 py-1.5 rounded-lg bg-primary/20 hover:bg-primary/30 border border-primary/30 text-primary text-[11px] font-bold tracking-widest uppercase transition-all duration-200 active:scale-[0.98]"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function KeybindCapture({
  label,
  icon,
  value,
  onChange,
}: {
  label: string;
  icon: string;
  value: string;
  onChange: (v: string) => void;
}) {
  const [capturing, setCapturing] = useState(false);
  const cancelRef = useRef(false);

  const stopCapture = useCallback(() => {
    cancelRef.current = true;
    setCapturing(false);
  }, []);

  useEffect(() => {
    if (!capturing) return;

    cancelRef.current = false;

    const handler = (e: KeyboardEvent) => {
      if (cancelRef.current) return;
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        stopCapture();
        return;
      }

      const code = e.code || "";
      const key = e.key;

      // Media keys (browsers report them in e.key; e.code may be empty)
      const mediaMap: Record<string, string> = {
        MediaPlayPause: "MediaPlayPause",
        MediaTrackNext: "MediaNextTrack",
        MediaTrackPrevious: "MediaPrevTrack",
        MediaStop: "MediaStop",
      };
      if (mediaMap[key] || mediaMap[code]) {
        onChange(mediaMap[key] || mediaMap[code]);
        setCapturing(false);
        return;
      }

      // Ignore bare modifier presses
      if (["Control", "Shift", "Alt", "Meta"].includes(key)) return;

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.shiftKey) parts.push("Shift");
      if (e.altKey) parts.push("Alt");
      if (e.metaKey) parts.push("Win");

      parts.push(codeToKey(code) || key);
      const combo = parts.join("+");
      onChange(combo);
      setCapturing(false);
    };

    const blurHandler = () => {
      // Small delay so we don't cancel on focus loss before keyup
      setTimeout(() => {
        if (!cancelRef.current) stopCapture();
      }, 200);
    };

    document.addEventListener("keydown", handler, true);
    window.addEventListener("blur", blurHandler);

    return () => {
      document.removeEventListener("keydown", handler, true);
      window.removeEventListener("blur", blurHandler);
    };
  }, [capturing, onChange, stopCapture]);

  const displayValue = formatCombo(value);

  return (
    <div className="flex items-center justify-between px-3 py-3 rounded-lg bg-surface-container-high/40 border border-primary/10">
      <div className="flex items-center gap-2.5">
        <span className="material-symbols-outlined text-[18px] text-primary/70">{icon}</span>
        <span className="font-label text-[12px] text-on-surface font-bold">{label}</span>
      </div>
      <div className="flex items-center gap-2">
        {capturing ? (
          <button
            onClick={stopCapture}
            className="px-3 py-1.5 rounded-lg bg-red-500/20 border border-red-500/30 text-red-400 text-[11px] font-bold tracking-wider animate-pulse cursor-pointer"
          >
            Press keys...
          </button>
        ) : (
          <>
            <span className="font-mono text-[11px] text-on-surface-variant/70 min-w-[80px] text-right">
              {displayValue}
            </span>
            <button
              onClick={() => setCapturing(true)}
              className="px-3 py-1.5 rounded-lg bg-primary/15 hover:bg-primary/25 border border-primary/20 text-primary text-[10px] font-bold tracking-wider transition-all duration-200 active:scale-[0.97]"
            >
              CAPTURE
            </button>
          </>
        )}
      </div>
    </div>
  );
}
