'use client';

import Link from 'next/link';
import { FaArrowRight, FaBolt, FaBrain, FaComments, FaLayerGroup, FaWandMagicSparkles } from 'react-icons/fa6';

interface AskMiaEntryCardProps {
  tokenAddress: string;
  tokenLabel: string;
  runId?: string | null;
}

export function AskMiaEntryCard({ tokenAddress, tokenLabel, runId }: AskMiaEntryCardProps) {
  const params = new URLSearchParams({
    q: tokenAddress,
    label: tokenLabel,
  });
  if (runId) {
    params.set('run', runId);
  }
  const href = `/mia/ask?${params.toString()}`;

  return (
    <section
      className="relative overflow-hidden rounded-[1.75rem] border p-5 shadow-[0_30px_80px_rgba(8,12,24,0.34)] md:p-7"
      style={{ background: 'linear-gradient(135deg, rgba(111,141,255,0.22), rgba(12,18,28,0.98) 55%, rgba(12,28,24,0.96))', borderColor: 'rgba(148,160,194,0.16)' }}
    >
      <div
        className="pointer-events-none absolute -right-16 top-0 h-44 w-44 rounded-full blur-3xl"
        style={{ background: 'rgba(111,141,255,0.22)' }}
      />
      <div
        className="pointer-events-none absolute bottom-0 left-10 h-28 w-28 rounded-full blur-3xl"
        style={{ background: 'rgba(0,255,163,0.1)' }}
      />

      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="relative">
          <div className="flex items-center gap-2">
            <span style={{ color: 'var(--primary)' }}>
              <FaBrain size={14} />
            </span>
            <p className="text-[10px] font-bold uppercase tracking-[0.22em]" style={{ color: 'var(--outline)' }}>
              Ask MIA
            </p>
          </div>
          <h2 className="mt-2 font-headline text-xl font-bold tracking-tight">
            Open the chat copilot for {tokenLabel}.
          </h2>
          <p className="mt-2 max-w-3xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
            Switch from report view to a dedicated chat workspace. Ask direct questions, see the internal tool activity, and read MIA&apos;s answer in a format closer to a real copilot than a static card.
          </p>

          <div className="mt-4 flex flex-wrap gap-2">
            <CapabilityPill icon={<FaLayerGroup size={12} />} label="Grounded internal tools" />
            <CapabilityPill icon={<FaBolt size={12} />} label="Structured plain-language answer" />
            <CapabilityPill icon={<FaWandMagicSparkles size={12} />} label="Visible thinking lane" />
          </div>
        </div>
        <div
          className="relative rounded-2xl border px-4 py-4 text-sm shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]"
          style={{ background: 'rgba(111,141,255,0.14)', borderColor: 'rgba(111,141,255,0.22)' }}
        >
          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
            Workspace mode
          </p>
          <p className="mt-1 font-semibold">Chat view with tool activity and grounded answers.</p>
          <p className="mt-2 max-w-xs text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
            Best when the report tells you what happened, but you still need to ask why it matters.
            {runId ? ' The active run will stay attached when you open chat.' : ''}
          </p>
        </div>
      </div>

      <div className="relative mt-6 grid gap-3 lg:grid-cols-[minmax(18rem,20rem)_1fr]">
        <Link
          href={href}
          data-testid="ask-mia-open-chat"
          className="inline-flex min-h-[4.75rem] items-center justify-between rounded-2xl border px-5 py-4 text-left text-sm font-bold uppercase tracking-[0.16em] shadow-[0_22px_40px_rgba(79,110,255,0.26)]"
          style={{ background: 'linear-gradient(135deg, rgba(111,141,255,1), rgba(124,147,255,0.92))', color: 'var(--on-primary-container)', borderColor: 'rgba(111,141,255,0.28)' }}
        >
          <span className="flex items-center gap-3">
            <FaComments size={15} />
            <span className="flex flex-col text-left normal-case tracking-normal">
              <span className="text-xs font-bold uppercase tracking-[0.16em] opacity-80">Ask MIA</span>
              <span className="mt-1 text-sm font-semibold">Open Ask MIA Chat</span>
            </span>
          </span>
          <FaArrowRight size={13} />
        </Link>

        <div className="grid gap-3 sm:grid-cols-3">
          <FeatureTile
            title="Ask for clarity"
            text="Get a clean explanation when the report still feels dense."
          />
          <FeatureTile
            title="See internal reads"
            text="Watch which market, risk, or builder tools MIA touched before answering."
          />
          <FeatureTile
            title="Decide faster"
            text="Best for organic versus manufactured flow, builder comparison, and next-step questions."
          />
        </div>
      </div>
    </section>
  );
}

function CapabilityPill({ icon, label }: { icon: React.ReactNode; label: string }) {
  return (
    <span
      className="inline-flex items-center gap-2 rounded-full border px-3 py-2 text-[11px] font-semibold"
      style={{ background: 'rgba(255,255,255,0.035)', borderColor: 'rgba(148,160,194,0.14)', color: 'var(--on-surface)' }}
    >
      <span style={{ color: 'var(--primary)' }}>{icon}</span>
      {label}
    </span>
  );
}

function FeatureTile({ title, text }: { title: string; text: string }) {
  return (
    <div
      className="rounded-2xl border px-4 py-4 text-sm shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]"
      style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(148,160,194,0.14)' }}
    >
      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {title}
      </p>
      <p className="mt-2 leading-7" style={{ color: 'var(--on-surface-variant)' }}>
        {text}
      </p>
    </div>
  );
}
