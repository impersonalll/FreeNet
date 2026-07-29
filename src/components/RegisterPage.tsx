export default function RegisterPage() {
  return (
    <div className="flex-1 flex flex-col items-center justify-center relative w-full px-12">
      <div className="absolute inset-0 bg-gradient-to-b from-primary/5 via-transparent to-transparent pointer-events-none" />

      <div className="w-full max-w-[500px] z-10">
        <div className="text-center mb-12">
          <h2 className="font-headline text-headline-sm text-on-surface tracking-[0.2em] font-bold uppercase mb-4">
            REGISTER
          </h2>
          <p className="font-body text-body-md text-on-surface-variant opacity-80">
            Create your FREENET account to sync settings across devices.
          </p>
        </div>

        <div className="space-y-5">
          <div>
            <label className="font-label text-label-md text-on-surface-variant tracking-[0.08em] font-bold uppercase block mb-2">
              Username
            </label>
            <input
              type="text"
              placeholder="Enter username"
              className="w-full bg-surface-container/80 border border-white/10 rounded-xl px-4 py-3 font-body text-body-md text-on-surface placeholder:text-outline/40 focus:outline-none focus:border-primary/50 focus:shadow-[0_0_20px_rgba(188,19,254,0.15)] transition-all duration-300"
            />
          </div>

          <div>
            <label className="font-label text-label-md text-on-surface-variant tracking-[0.08em] font-bold uppercase block mb-2">
              Email
            </label>
            <input
              type="email"
              placeholder="Enter email"
              className="w-full bg-surface-container/80 border border-white/10 rounded-xl px-4 py-3 font-body text-body-md text-on-surface placeholder:text-outline/40 focus:outline-none focus:border-primary/50 focus:shadow-[0_0_20px_rgba(188,19,254,0.15)] transition-all duration-300"
            />
          </div>

          <div>
            <label className="font-label text-label-md text-on-surface-variant tracking-[0.08em] font-bold uppercase block mb-2">
              Password
            </label>
            <input
              type="password"
              placeholder="Enter password"
              className="w-full bg-surface-container/80 border border-white/10 rounded-xl px-4 py-3 font-body text-body-md text-on-surface placeholder:text-outline/40 focus:outline-none focus:border-primary/50 focus:shadow-[0_0_20px_rgba(188,19,254,0.15)] transition-all duration-300"
            />
          </div>

          <button className="w-full mt-4 bg-gradient-to-r from-primary-container to-tertiary-container text-on-primary-container font-label text-label-md tracking-[0.1em] font-bold uppercase py-3.5 rounded-xl hover:shadow-[0_0_30px_rgba(188,19,254,0.3)] active:scale-[0.98] transition-all duration-300">
            Create Account
          </button>
        </div>
      </div>
    </div>
  );
}
