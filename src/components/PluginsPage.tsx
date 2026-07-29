export default function PluginsPage() {
  const plugins = [
    {
      name: "Telegram Optimizer",
      version: "1.2.0",
      description: "Enhanced connection routing for Telegram",
      icon: "send",
      active: true,
    },
    {
      name: "YouTube Unblocker",
      version: "2.0.1",
      description: "Bypass regional restrictions on video content",
      icon: "play_circle",
      active: true,
    },
    {
      name: "Discord Gateway",
      version: "1.5.3",
      description: "Optimized WebSocket relay for Discord voice",
      icon: "headset_mic",
      active: false,
    },
    {
      name: "Game Booster",
      version: "0.9.2",
      description: "Low-latency UDP tunneling for online gaming",
      icon: "sports_esports",
      active: false,
    },
  ];

  return (
    <div className="flex-1 flex flex-col items-center justify-center relative w-full px-12">
      <div className="absolute inset-0 bg-gradient-to-b from-secondary-container/5 via-transparent to-transparent pointer-events-none" />

      <div className="w-full max-w-[700px] z-10">
        <div className="text-center mb-12">
          <h2 className="font-headline text-headline-sm text-on-surface tracking-[0.2em] font-bold uppercase mb-4">
            PLUGINS
          </h2>
          <p className="font-body text-body-md text-on-surface-variant opacity-80">
            Modular extensions for protocol-specific optimizations.
          </p>
        </div>

        <div className="space-y-4">
          {plugins.map((plugin) => (
            <PluginCard key={plugin.name} {...plugin} />
          ))}
        </div>
      </div>
    </div>
  );
}

function PluginCard({
  name,
  version,
  description,
  icon,
  active,
}: {
  name: string;
  version: string;
  description: string;
  icon: string;
  active: boolean;
}) {
  return (
    <div className="glass-card rounded-2xl p-5 border border-white/10 bg-surface/30 liquid-blur hover:bg-white/5 transition-all duration-300 cursor-pointer group">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div
            className={`w-10 h-10 rounded-xl flex items-center justify-center border transition-colors ${
              active
                ? "bg-primary/10 border-primary/30 group-hover:border-primary/50"
                : "bg-surface-container-high/50 border-white/10 group-hover:border-white/20"
            }`}
          >
            <span
              className={`material-symbols-outlined text-[22px] ${
                active ? "text-primary" : "text-outline"
              }`}
            >
              {icon}
            </span>
          </div>
          <div>
            <div className="flex items-center gap-3">
              <h3 className="font-label text-label-md text-on-surface tracking-[0.05em] font-bold">
                {name}
              </h3>
              <span className="text-[10px] font-label tracking-wider text-outline/60 bg-surface-container-high/50 px-2 py-0.5 rounded-full">
                v{version}
              </span>
            </div>
            <p className="font-body text-[13px] text-on-surface-variant opacity-70 mt-0.5">
              {description}
            </p>
          </div>
        </div>
        <div
          className={`px-3 py-1 rounded-full font-label text-[10px] tracking-[0.1em] font-bold uppercase ${
            active
              ? "bg-green-500/15 text-green-400 border border-green-500/30"
              : "bg-surface-container-high/50 text-outline/60 border border-white/5"
          }`}
        >
          {active ? "Active" : "Inactive"}
        </div>
      </div>
    </div>
  );
}
