import { useState, useEffect, useRef, useCallback, createContext, useContext } from "react";
import { createPortal } from "react-dom";

export type ToastKind = "success" | "error" | "info" | "warning";

interface ToastItem {
  id: number;
  kind: ToastKind;
  title: string;
  message?: string;
}

interface ToastApi {
  show: (kind: ToastKind, title: string, message?: string) => void;
  success: (title: string, message?: string) => void;
  error: (title: string, message?: string) => void;
  info: (title: string, message?: string) => void;
  warning: (title: string, message?: string) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

export function useToast(): ToastApi {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within ToastProvider");
  return ctx;
}

const KIND_ICON: Record<ToastKind, string> = {
  success: "check_circle",
  error: "error",
  info: "info",
  warning: "warning",
};

const KIND_COLOR: Record<ToastKind, string> = {
  success: "text-green-400",
  error: "text-red-400",
  info: "text-primary",
  warning: "text-yellow-400",
};

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const nextId = useRef(1);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const show = useCallback(
    (kind: ToastKind, title: string, message?: string) => {
      const id = nextId.current++;
      setToasts((prev) => [...prev, { id, kind, title, message }]);
      setTimeout(() => dismiss(id), 4500);
    },
    [dismiss]
  );

  const api: ToastApi = {
    show,
    success: (t, m) => show("success", t, m),
    error: (t, m) => show("error", t, m),
    info: (t, m) => show("info", t, m),
    warning: (t, m) => show("warning", t, m),
  };

  return (
    <ToastContext.Provider value={api}>
      {children}
      {createPortal(
        <div className="fixed top-5 right-5 z-[500] flex flex-col gap-2.5 pointer-events-none">
          {toasts.map((t) => (
            <div
              key={t.id}
              className="pointer-events-auto w-[340px] rounded-xl border border-white/10 bg-[#0e021d]/95 glass-modal shadow-[0_16px_40px_rgba(0,0,0,0.6)] overflow-hidden animate-modal-in"
            >
              <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent" />
              <div className="flex items-start gap-3 px-4 py-3.5">
                <span className={`material-symbols-outlined text-[20px] shrink-0 mt-0.5 ${KIND_COLOR[t.kind]}`}>
                  {KIND_ICON[t.kind]}
                </span>
                <div className="flex-1 min-w-0">
                  <p className="font-label text-[12px] text-on-surface tracking-[0.03em] font-bold">
                    {t.title}
                  </p>
                  {t.message && (
                    <p className="font-body text-[11px] text-on-surface-variant opacity-70 mt-0.5 break-words leading-relaxed">
                      {t.message}
                    </p>
                  )}
                </div>
                <button
                  onClick={() => dismiss(t.id)}
                  className="w-6 h-6 rounded-full flex items-center justify-center text-on-surface-variant hover:text-on-surface hover:bg-white/10 transition-all duration-200 shrink-0"
                >
                  <span className="material-symbols-outlined text-[15px]">close</span>
                </button>
              </div>
            </div>
          ))}
        </div>,
        document.body
      )}
    </ToastContext.Provider>
  );
}
