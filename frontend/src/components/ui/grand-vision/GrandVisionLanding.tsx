import Image from 'next/image';
import Link from 'next/link';

type Accent = 'primary' | 'secondary' | 'tertiary';

type CapabilityCard = {
  title: string;
  text: string;
  label: string;
  icon: string;
  accent: Accent;
};

type WorkflowStep = {
  title: string;
  text: string;
  icon: string;
  accent?: Accent | 'solid';
  caption?: string;
};

type ProofCard = {
  title: string;
  text: string;
  label: string;
};

type StackCard = {
  title: string;
  label: string;
  text: string;
  icon: string;
  accent?: 'primary';
};

const navItems = [
  { label: 'Workflow', href: '#workflow', active: true },
  { label: 'Proof', href: '#proof' },
  { label: 'Under the Hood', href: '#under-the-hood' },
  { label: 'Roadmap', href: '#roadmap' },
  { label: 'Contact', href: '#footer' },
];

const evidencePoints = [
  'Persistent investigation runs, not one-shot prompts',
  'AI score, monitoring, and deep research inside one continuity layer',
  'Evidence gathered before synthesis, and trace kept after synthesis',
];

const traceItems = [
  {
    title: 'Wallet structure attached',
    detail: 'Linked funding routes and related addresses surfaced in the same run',
    age: '2s ago',
    icon: 'account_tree',
    accent: 'primary' as const,
  },
  {
    title: 'Historical analog matched',
    detail: 'Early launch behavior compared against similar prior windows',
    age: '11s ago',
    icon: 'timeline',
    accent: 'tertiary' as const,
  },
];

const secondaryCapabilities: CapabilityCard[] = [
  {
    label: 'Ask MIA',
    title: 'Ask grounded questions against the live run.',
    text: 'Ask MIA is tied to internal tools and run context, so answers stay anchored to actual token evidence instead of generic crypto commentary.',
    icon: 'forum',
    accent: 'primary',
  },
  {
    label: 'Monitoring Continuity',
    title: 'Keep the investigation alive after the first read.',
    text: 'Runs, watchlists, and missions keep state over time so monitoring, escalation, and downgrade are part of the same system.',
    icon: 'history',
    accent: 'tertiary',
  },
  {
    label: 'External Evidence',
    title: 'Escalate into deeper research only when it matters.',
    text: 'MIA can attach deeper research to the same token context instead of forcing users into a disconnected premium lane.',
    icon: 'hub',
    accent: 'secondary',
  },
];

const workflowJourney = [
  'Discover candidate',
  'Open investigation',
  'Score and explain',
  'Monitor through runs',
  'Escalate with Deep Research',
  'Preserve continuity',
];

const workflowSteps: WorkflowStep[] = [
  {
    title: '1. Discover',
    text: 'Pull live launch candidates into a triage surface that can immediately route into investigation.',
    icon: 'architecture',
  },
  {
    title: '2. Investigate',
    text: 'Collect wallet structure, deployer history, linked-launch evidence, and current token facts.',
    icon: 'database',
    accent: 'primary',
  },
  {
    title: '3. Score + Explain',
    text: 'Turn structured evidence into an AI read that explains why the token is active, weak, or worth deeper attention.',
    icon: 'hub',
    accent: 'tertiary',
    caption: 'AI',
  },
  {
    title: '4. Monitor',
    text: 'Keep the token alive inside runs, watchlists, and missions so the system can react when state changes.',
    icon: 'memory',
    accent: 'secondary',
  },
  {
    title: '5. Escalate',
    text: 'Attach Deep Research when the token deserves more source depth, context, and persistence.',
    icon: 'description',
    accent: 'solid',
  },
];

const differentiators = [
  'Run-based intelligence instead of page-based analytics',
  'Grounded AI instead of generic crypto summaries',
  'Monitoring continuity instead of isolated reads',
  'Deep research integrated into the same operational object',
];

const proofCards: ProofCard[] = [
  {
    label: 'Artifact 01',
    title: 'Investigation run',
    text: 'A persistent object with status, trigger, summary, and continuity over time.',
  },
  {
    label: 'Artifact 02',
    title: 'Run timeline',
    text: 'A readable event history of how the investigation changed and why it changed.',
  },
  {
    label: 'Artifact 03',
    title: 'AI score explanation',
    text: 'A clear explanation of whether the score is live, gated, or enriched by deep research.',
  },
  {
    label: 'Artifact 04',
    title: 'Deep research report',
    text: 'A saved report that can be attached back into the main token investigation.',
  },
  {
    label: 'Artifact 05',
    title: 'Watchlist + mission layer',
    text: 'Persistent monitoring surfaces that keep operator intent alive after the first read.',
  },
  {
    label: 'Artifact 06',
    title: 'Grounded Ask MIA',
    text: 'Focused questions answered from the same token evidence and run context.',
  },
];

const hostPoints = [
  'AI-native research desk',
  'Persistent monitoring desk',
  'Foundation for an autonomous trading intelligence firm',
];

const stackCards: StackCard[] = [
  {
    title: 'Tool-Routed AI Workflows',
    label: 'Reasoning Layer',
    text: 'MIA coordinates internal tools first, so scoring and research stay grounded in real product evidence instead of generic prompt output.',
    icon: 'alt_route',
  },
  {
    title: 'Persistent Operational Memory',
    label: 'Run Layer',
    text: 'Runs, events, watchlists, and missions give the system memory, continuity, and state instead of disposable page views.',
    icon: 'paid',
    accent: 'primary',
  },
  {
    title: 'Escalation Into Depth',
    label: 'Research Layer',
    text: 'Deep research can enrich the same token context so a stronger read is attached to the same operational object, not split into a different product.',
    icon: 'stacked_line_chart',
  },
];

function SymbolIcon({ name, className = '' }: { name: string; className?: string }) {
  return <span className={`material-symbols-outlined ${className}`.trim()}>{name}</span>;
}

function accentClass(accent: Accent) {
  if (accent === 'secondary') return 'text-[#dfb7ff] border-[#dfb7ff]/20 shadow-[0_0_20px_rgba(223,183,255,0.05)]';
  if (accent === 'tertiary') return 'text-[#ffd59c] border-[#ffd59c]/20 shadow-[0_0_20px_rgba(255,213,156,0.05)]';
  return 'text-[#a4e6ff] border-[#a4e6ff]/20 shadow-[0_0_20px_rgba(164,230,255,0.05)]';
}

function badgeClass(accent: Accent) {
  if (accent === 'secondary') return 'text-[#dfb7ff] bg-[#dfb7ff]/10 border-[#dfb7ff]/20';
  if (accent === 'tertiary') return 'text-[#ffd59c] bg-[#ffd59c]/10 border-[#ffd59c]/20';
  return 'text-[#a4e6ff] bg-[#a4e6ff]/10 border-[#a4e6ff]/20';
}

function SectionHeading({
  title,
  subtitle,
  align = 'left',
}: {
  title: string;
  subtitle: string;
  align?: 'left' | 'center';
}) {
  return (
    <div className={align === 'center' ? 'mx-auto max-w-4xl text-center' : 'max-w-3xl'}>
      <h2 className="font-headline text-4xl font-extrabold tracking-tight text-[#e5e1e4] md:text-5xl">
        {title}
      </h2>
      <p className="mt-4 text-base leading-8 text-[#bbc9cf] md:text-lg">{subtitle}</p>
    </div>
  );
}

function SmallCapabilityCard({ card }: { card: CapabilityCard }) {
  return (
    <article className="rounded-[1.75rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-6 transition-colors duration-300 hover:bg-[#201f21] md:p-7">
      <div className="flex items-center justify-between gap-4">
        <div
          className={`inline-flex h-14 w-14 items-center justify-center rounded-2xl border bg-[#131315] ${accentClass(card.accent)}`}
        >
          <SymbolIcon name={card.icon} className="text-[1.9rem] leading-none" />
        </div>
        <span className={`rounded-full border px-3 py-1 font-mono text-[10px] uppercase tracking-[0.12em] ${badgeClass(card.accent)}`}>
          {card.label}
        </span>
      </div>
      <h3 className="mt-6 font-headline text-2xl font-bold tracking-tight text-[#e5e1e4]">
        {card.title}
      </h3>
      <p className="mt-4 text-sm leading-7 text-[#bbc9cf]">{card.text}</p>
    </article>
  );
}

function ProofCardItem({ card }: { card: ProofCard }) {
  return (
    <article className="rounded-[1.5rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-5 transition-colors duration-300 hover:bg-[#201f21]">
      <div className="mb-4 rounded-2xl border border-[#a4e6ff]/12 bg-[linear-gradient(180deg,rgba(164,230,255,0.08),rgba(255,255,255,0.02))] p-4">
        <div className="mb-3 flex items-center justify-between text-[10px] uppercase tracking-[0.16em] text-[#859399]">
          <span>{card.label}</span>
          <span className="inline-flex items-center gap-1 text-[#a4e6ff]">
            <span className="h-1.5 w-1.5 rounded-full bg-[#a4e6ff]" />
            Verified
          </span>
        </div>
        <div className="space-y-2">
          <div className="h-2.5 w-3/4 rounded-full bg-[#353437]" />
          <div className="h-2.5 w-full rounded-full bg-[#2a2a2c]" />
          <div className="h-2.5 w-2/3 rounded-full bg-[#353437]" />
        </div>
      </div>
      <h3 className="font-headline text-xl font-bold tracking-tight text-[#e5e1e4]">{card.title}</h3>
      <p className="mt-3 text-sm leading-7 text-[#bbc9cf]">{card.text}</p>
    </article>
  );
}

function StackPanel({ card, className = '' }: { card: StackCard; className?: string }) {
  const iconTone =
    card.accent === 'primary'
      ? 'bg-[#a4e6ff]/10 border-[#a4e6ff]/30 text-[#a4e6ff]'
      : 'bg-[#131315] border-[#3c494e]/20 text-[#e5e1e4]';
  const titleTone = card.accent === 'primary' ? 'text-[#a4e6ff]' : 'text-[#e5e1e4]';
  const labelTone = card.accent === 'primary' ? 'text-[#a4e6ff]/70' : 'text-[#859399]';
  const borderTone = card.accent === 'primary' ? 'border-[#a4e6ff]/20' : 'border-[#3c494e]/30';

  return (
    <article
      className={`absolute w-full max-w-[26rem] rounded-[1.75rem] border bg-[#353437]/40 p-6 shadow-[0_20px_60px_rgba(0,0,0,0.5)] backdrop-blur-2xl transition-transform duration-500 ${borderTone} ${className}`}
    >
      <div className="mb-4 flex items-center gap-4">
        <div className={`inline-flex h-10 w-10 items-center justify-center rounded-xl border ${iconTone}`}>
          <SymbolIcon name={card.icon} className="text-[1.4rem] leading-none" />
        </div>
        <h3 className={`flex flex-col font-headline text-lg font-bold ${titleTone}`}>
          {card.title}
          <span className={`font-mono text-[10px] font-normal uppercase tracking-[0.08em] ${labelTone}`}>
            {card.label}
          </span>
        </h3>
      </div>
      <p className="text-sm leading-7 text-[#bbc9cf]">{card.text}</p>
    </article>
  );
}

export default function GrandVisionLanding() {
  return (
    <div className="min-h-screen overflow-x-hidden bg-[#131315] text-[#e5e1e4] selection:bg-[#00d1ff] selection:text-[#003543]">
      <nav className="sticky top-0 z-50 w-full bg-gradient-to-b from-[#131315] via-[#131315]/80 to-transparent backdrop-blur-3xl shadow-2xl shadow-[#a4e6ff]/5">
        <div className="mx-auto flex min-h-20 w-full max-w-7xl items-center justify-between px-6">
          <div className="flex items-center gap-2 font-headline text-2xl font-black tracking-tighter text-[#e5e1e4]">
            MIA
            <span className="rounded border border-[#a4e6ff]/20 bg-[#353437]/50 px-2 py-0.5 font-mono text-[10px] text-[#a4e6ff]">
              OS v2.4
            </span>
          </div>

          <div className="hidden items-center gap-8 md:flex">
            {navItems.map((item) => (
              <a
                key={item.label}
                href={item.href}
                className={
                  item.active
                    ? "border-b-2 border-[#a4e6ff] pb-1 font-headline text-sm font-bold tracking-tight text-[#a4e6ff]"
                    : 'font-headline text-sm font-bold tracking-tight text-[#bbc9cf] transition-colors hover:text-[#e5e1e4]'
                }
              >
                {item.label}
              </a>
            ))}
          </div>

          <Link
            href="/app"
            className="inline-flex items-center gap-2 rounded-full bg-[#00d1ff] px-6 py-2.5 font-headline text-sm font-bold tracking-tight text-[#003543] transition-transform duration-300 hover:scale-105 active:scale-95"
          >
            <span className="h-2 w-2 rounded-full bg-[#003543]" />
            APP
          </Link>
        </div>
      </nav>

      <main>
        <section className="relative overflow-hidden px-6 pb-24 pt-32 lg:px-8 lg:pb-32 lg:pt-40">
          <div className="pointer-events-none absolute right-0 top-0 h-[800px] w-[800px] translate-x-1/4 -translate-y-1/2 rounded-full bg-[#a4e6ff]/5 blur-[120px]" />
          <div className="pointer-events-none absolute bottom-0 left-0 h-[600px] w-[600px] -translate-x-1/4 translate-y-1/2 rounded-full bg-[#dfb7ff]/5 blur-[100px]" />

          <div className="relative z-10 mx-auto grid max-w-7xl grid-cols-1 items-center gap-16 lg:grid-cols-12">
            <div className="flex flex-col items-start gap-8 lg:col-span-6">
              <h1 className="font-headline text-5xl font-extrabold leading-[1.05] tracking-tight text-[#e5e1e4] md:text-6xl lg:text-7xl">
                MIA <span className="text-[#bbc9cf]">—</span> The{' '}
                <span className="bg-[linear-gradient(135deg,#a4e6ff,#00d1ff)] bg-clip-text text-transparent">
                  Autonomous Research Desk
                </span>
                <br />
                for Chaotic Onchain Launches.
              </h1>

              <p className="max-w-2xl text-lg leading-8 text-[#bbc9cf]">
                MIA is building the first operating wedge for an AI-native trading intelligence
                firm. The MVP already turns live launches into persistent investigations with AI
                scoring, monitoring continuity, deep research, and visible evidence layers.
              </p>

              <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.18em] text-[#a4e6ff]">
                Evidence-first. Run-based. Built to grow into an autonomous company layer.
              </p>

              <div className="flex w-full flex-col gap-4 pt-2 sm:flex-row">
                <Link
                  href="/app"
                  className="inline-flex items-center justify-center rounded-full bg-[#00d1ff] px-8 py-4 font-headline text-sm font-bold text-[#003543] shadow-[0_0_40px_rgba(164,230,255,0.2)] transition-colors hover:bg-[#4cd6ff]"
                >
                  Open Live Demo
                </Link>
                <a
                  href="#workflow"
                  className="inline-flex items-center justify-center gap-2 rounded-full border border-[#a4e6ff]/15 bg-[#353437]/20 px-8 py-4 font-headline text-sm font-bold text-[#e5e1e4] backdrop-blur-2xl transition-colors hover:bg-[#353437]/40"
                >
                  View Research Flow
                  <SymbolIcon name="account_tree" className="text-base leading-none" />
                </a>
              </div>
            </div>

            <div className="relative lg:col-span-6">
              <div className="relative flex aspect-square w-full flex-col overflow-hidden rounded-[1.8rem] border border-[#3c494e]/20 bg-[#353437]/10 p-2 shadow-[0_0_80px_rgba(0,209,255,0.05)] backdrop-blur-3xl">
                <div className="flex h-8 items-center gap-2 rounded-t-2xl border-b border-[#3c494e]/20 bg-[#0e0e10]/50 px-4">
                  <span className="h-2.5 w-2.5 rounded-full bg-[#ffb4ab]/70" />
                  <span className="h-2.5 w-2.5 rounded-full bg-[#feb127]/70" />
                  <span className="h-2.5 w-2.5 rounded-full bg-[#a4e6ff]/70" />
                  <span className="ml-2 font-mono text-[10px] text-[#bbc9cf]">mia-orchestrator-ui</span>
                </div>

                <div className="relative min-h-[24rem] flex-1 overflow-hidden rounded-b-2xl">
                  <Image
                    src="/grand-vision-hero.png"
                    alt="MIA research workflow preview"
                    fill
                    priority
                    sizes="(max-width: 1024px) 100vw, 520px"
                    className="object-cover pt-10 opacity-80 mix-blend-screen"
                  />
                </div>

                <div className="absolute bottom-10 left-10 right-10 rounded-xl border border-[#a4e6ff]/20 bg-[#353437]/60 p-4 shadow-2xl backdrop-blur-2xl">
                  <div className="flex items-center justify-between border-b border-[#3c494e]/30 pb-2 font-mono text-[10px] text-[#bbc9cf]">
                  <div className="flex items-center gap-2 uppercase tracking-[0.12em] text-[#a4e6ff]">
                      <span className="h-2 w-2 rounded-full bg-[#a4e6ff]" />
                      Active Investigation Run
                    </div>
                    <span>Continuity preserved</span>
                  </div>
                  <div className="mt-3 flex items-center justify-between gap-4">
                    <div>
                      <p className="mb-1 font-mono text-xs text-[#bbc9cf]">
                        Dossier ID: <span className="text-[#e5e1e4]">#4092-AX</span>
                      </p>
                      <p className="font-headline text-xl font-bold text-[#a4e6ff]">runs, scores, and deep research attached</p>
                    </div>
                    <div className="flex items-end gap-1" aria-hidden="true">
                      <span className="h-6 w-1.5 rounded-sm bg-[#a4e6ff]/40" />
                      <span className="h-8 w-1.5 rounded-sm bg-[#a4e6ff]/60" />
                      <span className="h-5 w-1.5 rounded-sm bg-[#a4e6ff]/30" />
                      <span className="h-10 w-1.5 rounded-sm bg-[#a4e6ff]" />
                      <span className="h-7 w-1.5 rounded-sm bg-[#a4e6ff]/50" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        <section className="bg-[#0e0e10] px-6 py-28 lg:px-8">
          <div className="mx-auto max-w-5xl text-center">
            <SectionHeading
              align="center"
              title="Evidence First. Continuity Built In."
              subtitle="Fresh launches move too fast for shallow dashboards and disposable AI summaries. MIA organizes scoring, investigation, monitoring, and deeper research into one operating system that users and future agents can inspect."
            />

            <div className="mt-12 grid gap-4 md:grid-cols-3">
              {evidencePoints.map((item) => (
                <div
                  key={item}
                  className="rounded-[1.4rem] border border-[#3c494e]/20 bg-[#1c1b1d] px-5 py-5 text-left text-sm leading-7 text-[#bbc9cf]"
                >
                  {item}
                </div>
              ))}
            </div>
          </div>
        </section>

        <section id="execution" className="px-6 py-28 lg:px-8">
          <div className="mx-auto max-w-7xl">
            <div className="mb-16 max-w-3xl">
              <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#a4e6ff]">
                Capability Cards
              </p>
              <h2 className="mt-3 font-headline text-4xl font-extrabold tracking-tight text-[#e5e1e4] md:text-5xl">
                The workflow is built around what an AI-native desk must actually do.
              </h2>
              <p className="mt-4 text-base leading-8 text-[#bbc9cf] md:text-lg">
                MIA helps a launch move from discovery into investigation, scoring, monitoring, and deeper research without breaking continuity.
              </p>
            </div>

            <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
              <article className="group relative flex min-h-[27rem] flex-col overflow-hidden rounded-[2rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-8 transition-colors duration-500 hover:bg-[#201f21]">
                <div className="absolute right-4 top-4 rounded border border-[#3c494e]/20 bg-[#201f21] px-2 py-1 font-mono text-[9px] uppercase tracking-[0.08em] text-[#bbc9cf]">
                  Discovery Surface
                </div>
                <div className="inline-flex h-14 w-14 items-center justify-center rounded-2xl border border-[#a4e6ff]/20 bg-[#131315] text-[#a4e6ff] shadow-[0_0_20px_rgba(164,230,255,0.05)]">
                  <SymbolIcon name="travel_explore" className="text-[1.9rem] leading-none" />
                </div>
                <h3 className="mt-6 font-headline text-3xl font-bold tracking-tight text-[#e5e1e4]">
                  Find a live candidate and open the run fast.
                </h3>
                <p className="mt-4 flex-grow text-sm leading-7 text-[#bbc9cf]">
                  Search new tokens, filter AI-scored and deep-researched candidates, and move straight into investigation.
                </p>
                <div className="mt-8 inline-flex items-center gap-2 font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-[#a4e6ff]">
                  Open surface
                  <SymbolIcon name="arrow_forward" className="text-base leading-none" />
                </div>
              </article>

              <article className="relative overflow-hidden rounded-[2rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-8 transition-colors duration-500 hover:bg-[#201f21] md:col-span-2">
                <div className="pointer-events-none absolute right-0 top-0 h-full w-1/2 bg-gradient-to-l from-[#a4e6ff]/5 to-transparent" />
                <div className="absolute right-4 top-4 flex items-center gap-2">
                  <span className="font-mono text-[9px] uppercase tracking-[0.08em] text-[#dfb7ff]">Deep Research</span>
                  <span className="rounded border border-[#dfb7ff]/30 bg-[#dfb7ff]/20 px-2 py-1 font-mono text-[10px] uppercase tracking-[0.08em] text-[#dfb7ff]">
                    Full Dossier
                  </span>
                </div>
                <div className="relative z-10 mt-4 flex h-full flex-col">
                  <div className="inline-flex h-14 w-14 items-center justify-center rounded-2xl border border-[#dfb7ff]/20 bg-[#131315] text-[#dfb7ff] shadow-[0_0_20px_rgba(223,183,255,0.05)]">
                    <SymbolIcon name="blur_on" className="text-[1.9rem] leading-none" />
                  </div>
                  <h3 className="mt-6 font-headline text-4xl font-bold tracking-tight text-[#e5e1e4]">
                    Run a full dossier, not just a longer summary.
                  </h3>
                  <p className="mt-4 max-w-xl text-sm leading-7 text-[#bbc9cf]">
                    Generate a structured report with run stages, visible trace, tool usage, and final synthesis.
                  </p>

                  <div className="mt-10 flex flex-col gap-3">
                    <div className="flex items-center justify-between font-mono text-[10px] text-[#bbc9cf]">
                      <span>Dossier assembly progress</span>
                      <span>67% Complete</span>
                    </div>
                    <div className="h-1.5 w-full overflow-hidden rounded-full bg-[#131315]">
                      <div className="relative h-full w-2/3 bg-gradient-to-r from-[#a4e6ff] to-[#dfb7ff]">
                        <div className="absolute inset-0 animate-[shimmer_2.2s_linear_infinite] bg-[linear-gradient(120deg,transparent_0%,rgba(255,255,255,0.2)_50%,transparent_100%)]" />
                      </div>
                    </div>
                    <div className="mt-1 flex flex-wrap gap-2 font-mono text-[9px] text-[#bbc9cf]">
                      <span className="rounded border border-[#3c494e]/20 bg-[#131315] px-2 py-1">
                        Run stages attached
                      </span>
                      <span className="rounded border border-[#3c494e]/20 bg-[#131315] px-2 py-1">
                        Tool usage visible
                      </span>
                      <span className="rounded border border-[#3c494e]/20 bg-[#131315] px-2 py-1">
                        Final synthesis grounded
                      </span>
                    </div>
                  </div>
                </div>
              </article>

              <article className="grid items-center gap-12 rounded-[2rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-8 transition-colors duration-500 hover:bg-[#201f21] md:col-span-3 md:grid-cols-[1.02fr_1fr]">
                <div>
                  <div className="mb-6 flex items-center gap-3">
                    <div className="inline-flex h-14 w-14 items-center justify-center rounded-2xl border border-[#ffd59c]/20 bg-[#131315] text-[#ffd59c] shadow-[0_0_20px_rgba(255,213,156,0.05)]">
                      <SymbolIcon name="visibility" className="text-[1.9rem] leading-none" />
                    </div>
                    <span className="inline-flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.1em] text-[#ffd59c]">
                      <span className="h-1.5 w-1.5 rounded-full bg-[#ffd59c]" />
                      Visible Trace
                    </span>
                  </div>
                  <h3 className="font-headline text-4xl font-bold tracking-tight text-[#e5e1e4]">
                    Inspect how the result was assembled.
                  </h3>
                  <p className="mt-4 max-w-xl text-sm leading-7 text-[#bbc9cf]">
                    The system shows steps, findings, and evidence flow instead of acting like a black box.
                  </p>
                  <button
                    type="button"
                    className="mt-6 inline-flex items-center gap-2 rounded-full border border-[#a4e6ff]/20 bg-[#353437]/40 px-6 py-3 font-mono text-xs text-[#a4e6ff] transition-colors hover:bg-[#353437]/60"
                  >
                    <SymbolIcon name="terminal" className="text-base leading-none" />
                    Open Trace View
                  </button>
                </div>

                <div className="rounded-3xl border border-[#3c494e]/20 bg-[#131315] p-6 font-mono">
                  {traceItems.map((item, index) => (
                    <div
                      key={item.title}
                      className={`flex items-start justify-between gap-4 px-1 py-4 ${index < traceItems.length - 1 ? 'border-b border-[#353437]' : ''}`}
                    >
                      <div className="flex items-start gap-4">
                        <div
                          className={`inline-flex h-10 w-10 items-center justify-center rounded-lg border ${item.accent === 'primary' ? 'border-[#a4e6ff]/20 bg-[#a4e6ff]/10 text-[#a4e6ff]' : 'border-[#ffd59c]/20 bg-[#ffd59c]/10 text-[#ffd59c]'}`}
                        >
                          <SymbolIcon name={item.icon} className="text-base leading-none" />
                        </div>
                        <div>
                          <p className="mb-1 text-xs font-bold text-[#e5e1e4]">{item.title}</p>
                          <p className="text-[10px] leading-5 text-[#bbc9cf]">{item.detail}</p>
                        </div>
                      </div>
                      <div className="flex flex-col items-end gap-1">
                        <span
                          className={`inline-flex items-center gap-1 rounded border px-2 py-0.5 text-[10px] font-bold ${item.accent === 'primary' ? 'border-[#a4e6ff]/20 bg-[#a4e6ff]/10 text-[#a4e6ff]' : 'border-[#ffd59c]/20 bg-[#ffd59c]/10 text-[#ffd59c]'}`}
                        >
                          <SymbolIcon name="check_circle" className="text-[10px] leading-none" />
                          Verified
                        </span>
                        <span className="text-[9px] text-[#859399]">{item.age}</span>
                      </div>
                    </div>
                  ))}
                </div>
              </article>
            </div>

            <div className="mt-8 grid gap-6 md:grid-cols-3">
              {secondaryCapabilities.map((card) => (
                <SmallCapabilityCard key={card.label} card={card} />
              ))}
            </div>
          </div>
        </section>

        <section id="workflow" className="border-y border-[#3c494e]/10 bg-[#0e0e10] px-6 py-28 lg:px-8">
          <div className="mx-auto max-w-7xl">
            <SectionHeading
              align="center"
              title="Traceable Investigation Workflow"
              subtitle="MIA turns launch analysis into a run-based workflow. Users and future agents start from discovery, inspect structure, score, monitor, deepen research, and preserve continuity in the same operational object."
            />

            <div className="mx-auto mt-10 flex max-w-5xl flex-wrap justify-center gap-3">
              {workflowJourney.map((item) => (
                <div
                  key={item}
                  className="rounded-full border border-[#3c494e]/20 bg-[#1c1b1d] px-4 py-2 font-mono text-[10px] uppercase tracking-[0.12em] text-[#bbc9cf]"
                >
                  {item}
                </div>
              ))}
            </div>

            <div className="relative mt-16">
              <div className="absolute left-0 top-8 hidden h-px w-full -translate-y-1/2 bg-gradient-to-r from-[#201f21] via-[#a4e6ff]/50 to-[#201f21] md:block" />
              <div className="relative grid grid-cols-1 gap-8 md:grid-cols-5">
                {workflowSteps.map((step) => {
                  const iconWrap =
                    step.accent === 'solid'
                      ? 'bg-[#00d1ff] text-[#003543] shadow-[0_0_30px_rgba(0,209,255,0.3)]'
                      : step.accent === 'tertiary'
                        ? 'h-20 w-20 -translate-y-2 border-[#ffd59c]/60 bg-[#353437]/40 text-[#ffd59c] shadow-[0_0_40px_rgba(255,213,156,0.2)] backdrop-blur-xl'
                        : step.accent === 'secondary'
                          ? 'text-[#dfb7ff]'
                          : step.accent === 'primary'
                            ? 'text-[#a4e6ff]'
                            : 'text-[#bbc9cf]';

                  return (
                    <div key={step.title} className="text-center">
                      <div
                        className={`relative mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-xl border border-[#3c494e]/50 bg-[#353437] shadow-[0_0_30px_rgba(0,0,0,0.5)] ${iconWrap}`}
                      >
                        {step.caption ? (
                          <span className="absolute -bottom-2 rounded bg-[#ffd59c] px-1.5 py-0.5 font-mono text-[8px] font-bold uppercase tracking-[0.08em] text-[#442b00]">
                            {step.caption}
                          </span>
                        ) : null}
                        <SymbolIcon
                          name={step.icon}
                          className={step.accent === 'tertiary' ? 'text-3xl leading-none' : 'text-[1.4rem] leading-none'}
                        />
                      </div>
                      <h3
                        className={`font-headline text-lg font-bold ${step.accent === 'tertiary' ? 'text-[#ffd59c]' : 'text-[#e5e1e4]'}`}
                      >
                        {step.title}
                      </h3>
                      <p className="mt-2 text-sm leading-6 text-[#bbc9cf]">{step.text}</p>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </section>

        <section className="px-6 py-28 lg:px-8">
          <div className="mx-auto grid max-w-7xl gap-16 lg:grid-cols-[0.95fr_1.05fr]">
            <div className="max-w-2xl">
              <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#a4e6ff]">
                Differentiation
              </p>
              <h2 className="mt-3 font-headline text-4xl font-extrabold tracking-tight text-[#e5e1e4] md:text-5xl">
                Not another AI wrapper
              </h2>
              <p className="mt-4 text-base leading-8 text-[#bbc9cf] md:text-lg">
                Many products in this category stop at charts, snippets, or generic AI summaries. MIA takes a different path: persistent runs, grounded scoring, monitoring loops, and deep research continuity inside one operating system.
              </p>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              {differentiators.map((item) => (
                <div
                  key={item}
                  className="rounded-[1.5rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-5 text-sm leading-7 text-[#bbc9cf]"
                >
                  {item}
                </div>
              ))}
            </div>
          </div>
        </section>

        <section id="proof" className="border-y border-[#3c494e]/10 bg-[#0e0e10] px-6 py-28 lg:px-8">
          <div className="mx-auto max-w-7xl">
            <SectionHeading
              title="Proof over prediction theater"
              subtitle="Chaotic launch markets do not need louder certainty. They need stronger evidence, clearer state transitions, and more inspectable research. MIA is built around that principle."
            />

            <div className="mt-14 grid gap-5 md:grid-cols-2 xl:grid-cols-3">
              {proofCards.map((card) => (
                <ProofCardItem key={card.title} card={card} />
              ))}
            </div>
          </div>
        </section>

        <section className="px-6 py-28 lg:px-8">
          <div className="mx-auto max-w-5xl text-center">
            <SectionHeading
              align="center"
              title="Built as an MVP for a much larger system"
              subtitle="The sprint output is a working AI-first research and monitoring desk. The long-term direction is an autonomous trading intelligence company where these workflows are increasingly handled by cooperating AI systems."
            />

            <div className="mt-12 grid gap-4 md:grid-cols-3">
              {hostPoints.map((item, index) => (
                <div
                  key={item}
                  className={`rounded-[1.5rem] border p-5 text-left ${index === 1 ? 'border-[#a4e6ff]/20 bg-[linear-gradient(180deg,rgba(164,230,255,0.06),rgba(255,255,255,0.015))]' : 'border-[#3c494e]/20 bg-[#1c1b1d]'}`}
                >
                  <p className="text-sm leading-7 text-[#bbc9cf]">{item}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section id="under-the-hood" className="relative overflow-hidden px-6 py-28 lg:px-8">
          <div className="pointer-events-none absolute left-1/2 top-1/2 h-[820px] w-[820px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-[#dfb7ff]/5 blur-[150px]" />

          <div className="relative z-10 mx-auto grid max-w-7xl items-center gap-20 lg:grid-cols-2">
            <div className="relative min-h-[32rem]">
              <StackPanel card={stackCards[0]} className="left-0 top-0 -rotate-3 hover:rotate-0" />
              <StackPanel card={stackCards[1]} className="right-0 top-1/3 rotate-2 hover:rotate-0" />
              <StackPanel card={stackCards[2]} className="bottom-0 left-10 -rotate-1 hover:rotate-0" />
            </div>

            <div className="max-w-2xl">
              <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#a4e6ff]">Under the Hood</p>
              <h2 className="mt-3 font-headline text-4xl font-extrabold tracking-tight text-[#e5e1e4] md:text-5xl">
                The interface stays calm. The system underneath is built for autonomous operations.
              </h2>
              <p className="mt-6 text-base leading-8 text-[#bbc9cf] md:text-lg">
                Behind the interface, MIA already coordinates grounded scoring, run continuity, monitoring state, and deeper research attachment. The long-term path is to turn this into a true AI workforce layer for trading intelligence.
              </p>
              <p className="mt-6 text-base leading-8 text-[#bbc9cf] md:text-lg">
                The output is not just an answer. It is an operational artifact with evidence, state, and continuity.
              </p>

              <ul className="mt-10 grid gap-4 font-mono text-sm text-[#e5e1e4]">
                <li className="flex items-center gap-3">
                  <SymbolIcon name="polyline" className="text-base leading-none text-[#a4e6ff]" />
                  Tool-routed AI workflows
                </li>
                <li className="flex items-center gap-3">
                  <SymbolIcon name="deployed_code" className="text-base leading-none text-[#a4e6ff]" />
                  Run-based research orchestration
                </li>
                <li className="flex items-center gap-3">
                  <SymbolIcon name="payments" className="text-base leading-none text-[#a4e6ff]" />
                  Deep research attachment and enrichment
                </li>
                <li className="flex items-center gap-3">
                  <SymbolIcon name="stacked_line_chart" className="text-base leading-none text-[#a4e6ff]" />
                  Monitoring loops and escalation state
                </li>
                <li className="flex items-center gap-3">
                  <SymbolIcon name="folder_supervised" className="text-base leading-none text-[#a4e6ff]" />
                  Evidence dossier assembly
                </li>
              </ul>
            </div>
          </div>
        </section>

        <section id="roadmap" className="border-y border-[#3c494e]/10 bg-[#0e0e10] px-6 py-28 lg:px-8">
          <div className="mx-auto max-w-7xl">
            <SectionHeading
              align="center"
              title="Roadmap Beyond The Sprint"
              subtitle="The current MVP proves the research and monitoring desk. The larger project is an AI-native company layer."
            />

            <div className="mt-14 grid gap-5 md:grid-cols-3">
              <article className="rounded-[1.5rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-6">
                <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-[#a4e6ff]">Phase 1</div>
                <h3 className="mt-3 font-headline text-2xl font-bold text-[#e5e1e4]">AI Research Desk</h3>
                <p className="mt-3 text-sm leading-7 text-[#bbc9cf]">Discovery, investigation, grounded AI scoring, Ask MIA, deep research, and proof support.</p>
              </article>
              <article className="rounded-[1.5rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-6">
                <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-[#a4e6ff]">Phase 2</div>
                <h3 className="mt-3 font-headline text-2xl font-bold text-[#e5e1e4]">AI Monitoring Firm</h3>
                <p className="mt-3 text-sm leading-7 text-[#bbc9cf]">Mission routing, stronger memory, richer monitoring loops, escalation policies, and persistent operator continuity.</p>
              </article>
              <article className="rounded-[1.5rem] border border-[#3c494e]/20 bg-[#1c1b1d] p-6">
                <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-[#a4e6ff]">Phase 3</div>
                <h3 className="mt-3 font-headline text-2xl font-bold text-[#e5e1e4]">AI Trading Intelligence Company</h3>
                <p className="mt-3 text-sm leading-7 text-[#bbc9cf]">Bounded execution, treasury policy, specialized agents, and increasingly autonomous operation under guardrails.</p>
              </article>
            </div>
          </div>
        </section>

        <section className="px-6 pb-10 pt-8 lg:px-8 lg:pb-20">
          <div className="mx-auto max-w-6xl rounded-[2rem] border border-[#3c494e]/20 bg-[linear-gradient(180deg,rgba(164,230,255,0.06),rgba(255,255,255,0.015))] px-6 py-12 text-center md:px-10 md:py-16">
            <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#a4e6ff]">Final CTA</p>
            <h2 className="mt-3 font-headline text-4xl font-extrabold tracking-tight text-[#e5e1e4] md:text-5xl">
              Open the Agentic Workflow
            </h2>
            <p className="mx-auto mt-4 max-w-3xl text-base leading-8 text-[#bbc9cf] md:text-lg">
              MIA turns chaotic launches into structured AI operations today, and points toward an autonomous trading intelligence firm tomorrow.
            </p>
            <div className="mt-8 flex flex-col justify-center gap-4 sm:flex-row">
              <Link
                href="/app"
                className="inline-flex items-center justify-center rounded-full bg-[#00d1ff] px-8 py-4 font-headline text-sm font-bold text-[#003543] shadow-[0_0_40px_rgba(164,230,255,0.2)] transition-colors hover:bg-[#4cd6ff]"
              >
                Launch Demo
              </Link>
              <a
                href="https://github.com/pandu926/MIA/blob/main/docs/submission/dorahacks-submission-mia.md"
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center justify-center rounded-full border border-[#a4e6ff]/20 bg-[#353437]/20 px-8 py-4 font-headline text-sm font-bold text-[#e5e1e4] backdrop-blur-2xl transition-colors hover:bg-[#353437]/40"
              >
                View Submission
              </a>
            </div>
          </div>
        </section>
      </main>

      <footer id="footer" className="mt-20 border-t border-[#3c494e]/10 bg-[linear-gradient(180deg,rgba(53,52,55,0.12),rgba(28,27,29,0.92))] px-6 py-12 lg:px-8">
        <div className="mx-auto grid max-w-7xl gap-8 md:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)_auto] md:items-center">
            <div className="grid gap-3 text-center md:text-left">
              <div className="inline-flex items-center justify-center gap-2 font-headline text-lg font-bold text-[#e5e1e4] md:justify-start">
                MIA
                <span className="rounded bg-[#353437] px-1.5 py-0.5 font-mono text-[9px] text-[#bbc9cf]">SYSTEM NORMAL</span>
              </div>
              <p className="text-sm leading-6 text-[#bbc9cf]">
              AI-native research and monitoring operating system for onchain launches.
              </p>
            </div>

          <div className="flex flex-wrap justify-center gap-3">
            <a
              href="#workflow"
              className="inline-flex min-h-9 items-center justify-center rounded-full border border-[#3c494e]/25 bg-[#353437]/50 px-4 font-mono text-[11px] uppercase tracking-[0.16em] text-[#bbc9cf] transition-colors hover:border-[#a4e6ff]/20 hover:bg-[#353437]/70 hover:text-[#a4e6ff]"
            >
              Research Flow
            </a>
            <a
              href="#proof"
              className="inline-flex min-h-9 items-center justify-center rounded-full border border-[#3c494e]/25 bg-[#353437]/50 px-4 font-mono text-[11px] uppercase tracking-[0.16em] text-[#bbc9cf] transition-colors hover:border-[#a4e6ff]/20 hover:bg-[#353437]/70 hover:text-[#a4e6ff]"
            >
              Proof
            </a>
            <a
              href="#under-the-hood"
              className="inline-flex min-h-9 items-center justify-center rounded-full border border-[#3c494e]/25 bg-[#353437]/50 px-4 font-mono text-[11px] uppercase tracking-[0.16em] text-[#bbc9cf] transition-colors hover:border-[#a4e6ff]/20 hover:bg-[#353437]/70 hover:text-[#a4e6ff]"
            >
              Under the Hood
            </a>
            <a
              href="#roadmap"
              className="inline-flex min-h-9 items-center justify-center rounded-full border border-[#3c494e]/25 bg-[#353437]/50 px-4 font-mono text-[11px] uppercase tracking-[0.16em] text-[#bbc9cf] transition-colors hover:border-[#a4e6ff]/20 hover:bg-[#353437]/70 hover:text-[#a4e6ff]"
            >
              Roadmap
            </a>
            <a
              href="https://github.com/pandu926/MIA/blob/main/docs/submission/dorahacks-submission-mia.md"
              target="_blank"
              rel="noreferrer"
              className="inline-flex min-h-9 items-center justify-center rounded-full border border-[#3c494e]/25 bg-[#353437]/50 px-4 font-mono text-[11px] uppercase tracking-[0.16em] text-[#bbc9cf] transition-colors hover:border-[#a4e6ff]/20 hover:bg-[#353437]/70 hover:text-[#a4e6ff]"
            >
              Submission
            </a>
          </div>

          <div className="text-center font-mono text-[11px] uppercase tracking-[0.16em] text-[#bbc9cf] md:text-right">
            © 2026 MIA. Autonomous trading intelligence starts with evidence.
          </div>
        </div>
      </footer>
    </div>
  );
}
