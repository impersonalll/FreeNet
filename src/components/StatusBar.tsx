interface StatusBarProps {
  isRunning: boolean;
  activeBypass?: string | null;
}

export default function StatusBar({ isRunning, activeBypass }: StatusBarProps) {
  return (
    <footer className="h-12 bg-surface/50 liquid-blur border-t border-white/15 px-6 flex items-center justify-between shrink-0">
      <div className="flex items-center gap-4">
        <div
          className={`w-3 h-3 rounded-full ${
            isRunning
              ? "bg-green-400 shadow-[0_0_15px_rgba(74,222,128,0.9)]"
              : "bg-primary shadow-[0_0_15px_rgba(188,19,254,0.9)]"
          } status-dot`}
        />
        <span className="font-label text-label-md tracking-[0.2em] font-bold uppercase text-primary">
          {isRunning ? `Connected · ${activeBypass ?? "Bypass"}` : "Ready for connection"}
        </span>
      </div>
      <div className="flex items-center gap-8">
        <span className="font-label text-label-md text-outline/70 tracking-widest">
          v1.0.0-STABLE
        </span>
        <div className="h-4 w-[1px] bg-white/10" />
        <span className="font-label text-label-md text-on-surface-variant tracking-[0.2em] font-bold">
          {isRunning ? "SYSTEM ACTIVE" : "SYSTEM SECURE"}
        </span>
      </div>
    </footer>
  );
}
