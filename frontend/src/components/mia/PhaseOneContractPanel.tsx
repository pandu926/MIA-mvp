import { buildPhaseOneCapabilityLanes, MIA_PHASE_ONE_CAPABILITY } from '@/lib/mia-capability';

export function PhaseOneContractPanel() {
  const lanes = buildPhaseOneCapabilityLanes();

  return (
    <section
      className="rounded-2xl border p-5 md:p-6"
      style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
    >
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-[10px] font-bold uppercase tracking-[0.22em]" style={{ color: 'var(--primary)' }}>
            Phase 1 Contract
          </p>
          <h2 className="mt-2 font-headline text-2xl font-bold tracking-tight">Free decision first. Paid depth only when the setup deserves it.</h2>
          <p className="mt-2 max-w-3xl text-sm" style={{ color: 'var(--on-surface-variant)' }}>
            The MVP keeps the first door rich and complete. MIA gives a free workflow with decision, evidence, action route, and monitoring handoff, then offers a deeper third-party-backed dossier only when the setup deserves more conviction work.
          </p>
        </div>
        <div
          className="inline-flex rounded-full px-3 py-2 text-[11px] font-bold uppercase tracking-[0.18em]"
          style={{ background: 'rgba(105,137,255,0.1)', color: 'var(--primary)' }}
        >
          {MIA_PHASE_ONE_CAPABILITY.deepResearch.primaryResearchProvider}
        </div>
      </div>

      <div className="mt-5 grid gap-4 lg:grid-cols-2">
        {lanes.map((lane) => (
          <div
            key={lane.title}
            className="rounded-2xl border p-5"
            style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}
          >
            <div className="flex items-center justify-between gap-3">
              <h3 className="font-headline text-xl font-bold tracking-tight">{lane.title}</h3>
              <span
                className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                style={{
                  background: lane.accessLabel === 'Free lane' ? 'rgba(0,255,163,0.08)' : 'rgba(255,186,73,0.12)',
                  color: lane.accessLabel === 'Free lane' ? 'var(--secondary-container)' : 'var(--warning)',
                }}
              >
                {lane.accessLabel}
              </span>
            </div>

            <p className="mt-3 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
              {lane.summary}
            </p>

            <div className="mt-4 space-y-2">
              {lane.bullets.map((item) => (
                <div
                  key={item}
                  className="rounded-lg border px-3 py-2 text-sm"
                  style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'rgba(255,255,255,0.02)' }}
                >
                  {item}
                </div>
              ))}
            </div>

            <p className="mt-4 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
              {lane.footnote}
            </p>

            {lane.ctaLabel && (
              <div className="mt-4 inline-flex rounded-lg px-3 py-2 text-xs font-semibold" style={{ background: 'rgba(105,137,255,0.1)', color: 'var(--primary)' }}>
                {lane.ctaLabel}
              </div>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}
