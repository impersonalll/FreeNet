import { useRef, useEffect, useState } from "react";
import type { Page } from "../App";

interface NavBarProps {
  activePage: Page;
  onPageChange: (page: Page) => void;
}

export default function NavBar({ activePage, onPageChange }: NavBarProps) {
  const tabs: { id: Page; label: string }[] = [
    { id: "freenet", label: "FREENET" },
    { id: "bypass", label: "BYPASS" },
    { id: "plugins", label: "PLUGINS" },
  ];

  const containerRef = useRef<HTMLDivElement>(null);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const [indicator, setIndicator] = useState({ left: 0, width: 0 });

  useEffect(() => {
    const idx = tabs.findIndex((t) => t.id === activePage);
    const btn = tabRefs.current[idx];
    const container = containerRef.current;
    if (btn && container) {
      const containerRect = container.getBoundingClientRect();
      const btnRect = btn.getBoundingClientRect();
      setIndicator({
        left: btnRect.left - containerRect.left,
        width: btnRect.width,
      });
    }
  }, [activePage]);

  return (
    <div className="bg-surface-container-low/20 liquid-blur border-b border-white/10 flex justify-between items-center px-6 h-14 w-full shrink-0">
      <div ref={containerRef} className="relative flex items-center">
        <div
          className="absolute top-1 bottom-1 rounded-full bg-primary/15 border border-primary/25 backdrop-blur-md transition-all duration-300 ease-out shadow-[0_0_12px_rgba(188,19,254,0.15)]"
          style={{
            left: indicator.left,
            width: indicator.width,
          }}
        />

        <div className="relative flex p-1 gap-0.5">
          {tabs.map((tab, i) => (
            <button
              key={tab.id}
              ref={(el) => { tabRefs.current[i] = el; }}
              onClick={() => onPageChange(tab.id)}
              className={`relative z-10 px-5 py-1.5 rounded-full font-label text-label-md transition-colors duration-300 ${
                activePage === tab.id
                  ? "text-primary"
                  : "text-on-surface-variant hover:text-on-surface"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-center gap-4">
        <span className="font-label text-label-md text-outline/40 tracking-widest text-xs">
          v1.0.0
        </span>
        <button
          onClick={() => onPageChange(activePage === "settings" ? "freenet" : "settings")}
          className={`w-10 h-10 rounded-full flex items-center justify-center transition-colors duration-300 ${
            activePage === "settings"
              ? "text-primary bg-primary/15"
              : "text-on-surface-variant hover:text-on-surface hover:bg-white/10"
          }`}
        >
          <span className="material-symbols-outlined text-[20px]">settings</span>
        </button>
      </div>
    </div>
  );
}
