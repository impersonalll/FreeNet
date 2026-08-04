import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export default function TitleBar() {
  const window = getCurrentWindow();

  const minimize = () => {
    invoke("minimize_window");
  };

  const close = () => {
    invoke("hide_window");
  };

  const handleDoubleClick = () => {
    window.toggleMaximize();
  };

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={handleDoubleClick}
      className="bg-surface/40 liquid-blur border-b border-white/15 shadow-[inset_0_1px_0_rgba(255,255,255,0.1)] flex justify-between items-center px-6 h-14 w-full shrink-0"
    >
      <div className="flex items-center gap-2.5" data-tauri-drag-region>
        <div
          className="w-7 h-7 rounded-[9px] bg-gradient-to-br from-primary/30 to-primary/5 border border-primary/30 flex items-center justify-center shadow-[0_0_12px_rgba(188,19,254,0.35)]"
          data-tauri-drag-region
        >
          <span className="material-symbols-outlined text-[16px] text-primary" data-tauri-drag-region>
            bolt
          </span>
        </div>
        <span
          className="font-headline text-primary tracking-[0.25em] font-bold drop-shadow-[0_0_8px_rgba(188,19,254,0.6)] text-sm uppercase"
          data-tauri-drag-region
        >
          FREENET
        </span>
      </div>
      <div className="flex items-center gap-3" data-tauri-no-drag>
        <button
          onClick={minimize}
          className="text-on-surface-variant hover:bg-white/15 transition-all w-8 h-8 rounded-full flex items-center justify-center active:scale-90 duration-200"
        >
          <span className="material-symbols-outlined text-[18px]">remove</span>
        </button>
        <button
          onClick={close}
          className="text-on-surface-variant hover:bg-white/15 transition-all w-8 h-8 rounded-full flex items-center justify-center active:scale-90 duration-200"
        >
          <span className="material-symbols-outlined text-[18px]">close</span>
        </button>
      </div>
    </header>
  );
}
