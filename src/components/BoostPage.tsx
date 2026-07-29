export default function BoostPage() {
  return (
    <div className="flex-1 flex flex-col items-center justify-center relative w-full px-12">
      <div className="absolute inset-0 bg-gradient-to-b from-tertiary-container/5 via-transparent to-transparent pointer-events-none" />

      <div className="w-full max-w-[700px] z-10">
        <div className="text-center mb-12">
          <h2 className="font-headline text-headline-sm text-on-surface tracking-[0.2em] font-bold uppercase mb-4">
            BOOST MODE
          </h2>
          <p className="font-body text-body-md text-on-surface-variant opacity-80">
            Enhanced performance settings for optimized routing and latency
            reduction.
          </p>
        </div>

        <div className="space-y-4">
          <BoostCard
            title="Low Latency Mode"
            description="Reduces routing hops for real-time applications"
            icon="speed"
            enabled={false}
          />
          <BoostCard
            title="Anti-Throttle"
            description="Prevents ISP bandwidth throttling on streaming traffic"
            icon="thunderstorm"
            enabled={false}
          />
          <BoostCard
            title="DNS Shield"
            description="Encrypted DNS queries to prevent DNS poisoning"
            icon="shield"
            enabled={false}
          />
          <BoostCard
            title="Traffic Masquerade"
            description="Obfuscates traffic patterns to bypass deep packet inspection"
            icon="mask"
            enabled={false}
          />
        </div>
      </div>
    </div>
  );
}

function BoostCard({
  title,
  description,
  icon,
  enabled,
}: {
  title: string;
  description: string;
  icon: string;
  enabled: boolean;
}) {
  return (
    <div className="glass-card rounded-2xl p-5 border border-white/10 bg-surface/30 liquid-blur hover:bg-white/5 transition-all duration-300 cursor-pointer group">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center border border-primary/20 group-hover:border-primary/40 transition-colors">
            <span className="material-symbols-outlined text-primary text-[22px]">
              {icon}
            </span>
          </div>
          <div>
            <h3 className="font-label text-label-md text-on-surface tracking-[0.05em] font-bold">
              {title}
            </h3>
            <p className="font-body text-[13px] text-on-surface-variant opacity-70 mt-0.5">
              {description}
            </p>
          </div>
        </div>
        <div
          className={`w-12 h-6 rounded-full transition-all duration-300 relative cursor-pointer ${
            enabled
              ? "bg-primary-container shadow-[0_0_12px_rgba(188,19,254,0.4)]"
              : "bg-surface-container-high border border-white/10"
          }`}
        >
          <div
            className={`w-5 h-5 rounded-full bg-white absolute top-0.5 transition-all duration-300 ${
              enabled ? "left-[26px]" : "left-0.5"
            }`}
          />
        </div>
      </div>
    </div>
  );
}
