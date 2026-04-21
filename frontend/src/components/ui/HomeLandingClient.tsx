'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useMemo, useState } from 'react';
import {
  FaArrowRight,
  FaBell,
  FaBolt,
  FaChartLine,
  FaChevronLeft,
  FaChevronRight,
  FaCopy,
  FaMagnifyingGlass,
  FaRadio,
  FaWaveSquare,
} from 'react-icons/fa6';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { InvestigationOpsSummaryResponse, InvestigationRunSummary, TokenListResponse, TokenSummary } from '@/lib/types';

type FeedView = 'latest' | 'top_1h' | 'top_24h' | 'top_7d';
type DiscoverFilter = 'all' | 'ai_scored' | 'deep_research';
type DiscoverSort = 'feed' | 'tx';

const PAGE_SIZE = 12;

const FEED_VIEWS: Record<
  FeedView,
  {
    label: string;
    title: string;
    sort: 'newest' | 'activity';
    windowHours?: number;
  }
> = {
  latest: { label: 'Latest', title: 'Live Feed', sort: 'newest' },
  top_1h: { label: '1H', title: 'Top 1H', sort: 'activity', windowHours: 1 },
  top_24h: { label: '24H', title: 'Top 24H', sort: 'activity', windowHours: 24 },
  top_7d: { label: '7D', title: 'Top 7D', sort: 'activity', windowHours: 168 },
};

const SHELL = {
  ink: '#edf1ff',
  muted: '#94a0c2',
  body: '#adb6d0',
  blue: '#6f8dff',
  green: '#36efb6',
  yellow: '#ffd166',
  red: '#ff8080',
  panel: 'rgba(23,31,49,0.9)',
  panelSoft: 'rgba(18,24,38,0.9)',
  border: 'rgba(255,255,255,0.06)',
};

function shortAddress(value: string, head = 6, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

function formatNumber(value: number | null | undefined, digits = 2) {
  if (value === null || value === undefined || Number.isNaN(value)) return '0';
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: 0,
    maximumFractionDigits: digits,
  }).format(value);
}

function timeAgo(value: string) {
  const diff = Date.now() - new Date(value).getTime();
  const sec = Math.max(1, Math.floor(diff / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day}d ago`;
  return `${Math.floor(day / 7)}w ago`;
}

function toneForRisk(value: TokenSummary['risk_category']) {
  if (value === 'low') return { color: SHELL.green, bg: 'rgba(54,239,182,0.1)', label: 'LOW RISK' };
  if (value === 'high') return { color: SHELL.red, bg: 'rgba(255,128,128,0.12)', label: 'HIGH RISK' };
  return { color: SHELL.yellow, bg: 'rgba(255,209,102,0.1)', label: 'MEDIUM' };
}

function toneForRunStatus(status: string) {
  if (status === 'escalated') return { color: SHELL.yellow, bg: 'rgba(255,209,102,0.12)' };
  if (status === 'watching') return { color: SHELL.blue, bg: 'rgba(111,141,255,0.12)' };
  if (status === 'queued' || status === 'running') return { color: SHELL.ink, bg: 'rgba(255,255,255,0.06)' };
  return { color: SHELL.green, bg: 'rgba(54,239,182,0.12)' };
}

function displayScore(token: TokenSummary) {
  return Math.max(18, Math.min(96, Math.round(token.composite_score ?? 52)));
}

function feedMetric(token: TokenSummary, view: FeedView) {
  if (view === 'latest') {
    return {
      volumeValue: `${formatNumber(token.volume_bnb, 2)} BNB`,
      flowValue: `${token.buy_count}/${token.sell_count}`,
      hint: timeAgo(token.deployed_at),
    };
  }

  return {
    volumeValue: `${formatNumber(token.window_volume_bnb, 2)} BNB`,
    flowValue: `${token.window_buy_count ?? 0}/${token.window_sell_count ?? 0}`,
    hint: `${token.window_hours ?? FEED_VIEWS[view].windowHours}h window`,
  };
}

function ScoreRing({ score, size = 52, stroke = 4 }: { score: number; size?: number; stroke?: number }) {
  const radius = (size - stroke * 2) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (score / 100) * circumference;
  const color = score >= 75 ? SHELL.green : score >= 50 ? SHELL.blue : score >= 30 ? SHELL.yellow : SHELL.red;

  return (
    <div style={{ position: 'relative', width: size, height: size, flexShrink: 0 }}>
      <svg width={size} height={size} style={{ transform: 'rotate(-90deg)', position: 'absolute', inset: 0 }}>
        <circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke="rgba(255,255,255,0.07)" strokeWidth={stroke} />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={color}
          strokeWidth={stroke}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
        />
      </svg>
      <div
        style={{
          position: 'absolute',
          inset: 0,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color,
          fontFamily: "'Roboto Mono', monospace",
          fontSize: size * 0.22,
          fontWeight: 700,
        }}
      >
        {score}
      </div>
    </div>
  );
}

function LiveDot() {
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
      <span
        style={{
          width: 7,
          height: 7,
          borderRadius: '50%',
          background: SHELL.green,
          boxShadow: '0 0 0 0 rgba(54,239,182,0.4)',
        }}
      />
      <span
        style={{
          color: SHELL.green,
          fontSize: 10,
          letterSpacing: '0.14em',
          textTransform: 'uppercase',
          fontWeight: 800,
          fontFamily: "'Manrope', sans-serif",
        }}
      >
        Live
      </span>
    </span>
  );
}

function SectionLabel({
  title,
  action,
  href,
}: {
  title: string;
  action?: string;
  href?: string;
}) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 16px', marginBottom: 8 }}>
      <span
        style={{
          fontSize: 10,
          fontWeight: 800,
          color: SHELL.muted,
          letterSpacing: '0.18em',
          textTransform: 'uppercase',
          fontFamily: "'Manrope', sans-serif",
        }}
      >
        {title}
      </span>
      {action && href ? (
        <Link
          href={href}
          style={{
            fontSize: 10,
            color: SHELL.blue,
            letterSpacing: '0.1em',
            textTransform: 'uppercase',
            fontWeight: 800,
            fontFamily: "'Manrope', sans-serif",
          }}
        >
          {action}
        </Link>
      ) : null}
    </div>
  );
}

function QuickActionCard({
  href,
  icon,
  title,
  sub,
  color,
}: {
  href: string;
  icon: React.ReactNode;
  title: string;
  sub: string;
  color: string;
}) {
  return (
    <Link
      href={href}
      style={{
        display: 'block',
        background: SHELL.panel,
        border: `1px solid ${SHELL.border}`,
        borderRadius: 14,
        padding: '16px 14px',
      }}
    >
      <div style={{ color, fontSize: 22, marginBottom: 8 }}>{icon}</div>
      <div style={{ color, fontSize: 13, fontWeight: 800, fontFamily: "'Manrope', sans-serif" }}>{title}</div>
      <div style={{ marginTop: 3, color: SHELL.muted, fontSize: 11, fontFamily: "'Space Grotesk', sans-serif" }}>{sub}</div>
    </Link>
  );
}

function StatCard({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div
      style={{
        flex: 1,
        background: SHELL.panel,
        border: `1px solid ${SHELL.border}`,
        borderRadius: 12,
        padding: '12px 10px',
        textAlign: 'center',
      }}
    >
      <div style={{ fontSize: 22, fontWeight: 800, fontFamily: "'Roboto Mono', monospace", color, lineHeight: 1 }}>
        {value}
      </div>
      <div
        style={{
          fontSize: 9,
          color: SHELL.muted,
          marginTop: 4,
          fontFamily: "'Manrope', sans-serif",
          letterSpacing: '0.1em',
          textTransform: 'uppercase',
          fontWeight: 700,
        }}
      >
        {label}
      </div>
    </div>
  );
}

function FeedRow({
  token,
  view,
  onCopy,
}: {
  token: TokenSummary;
  view: FeedView;
  onCopy: (address: string) => void;
}) {
  const metric = feedMetric(token, view);
  const risk = toneForRisk(token.risk_category);
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: '12px 16px',
        borderBottom: '1px solid rgba(255,255,255,0.04)',
      }}
    >
      <ScoreRing score={displayScore(token)} size={48} stroke={3} />
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 3, flexWrap: 'wrap' }}>
          <span style={{ fontSize: 14, fontWeight: 900, fontFamily: "'Manrope', sans-serif", color: SHELL.ink }}>
            {token.symbol ?? token.name ?? shortAddress(token.contract_address)}
          </span>
          <span
            style={{
              color: risk.color,
              background: risk.bg,
              borderRadius: 4,
              padding: '2px 6px',
              fontSize: 9,
              fontWeight: 800,
              letterSpacing: '0.1em',
              textTransform: 'uppercase',
              fontFamily: "'Manrope', sans-serif",
            }}
          >
            {risk.label}
          </span>
          {token.ai_scored ? (
            <span
              style={{
                color: SHELL.blue,
                background: 'rgba(111,141,255,0.14)',
                borderRadius: 4,
                padding: '2px 6px',
                fontSize: 9,
                fontWeight: 800,
                letterSpacing: '0.1em',
                textTransform: 'uppercase',
                fontFamily: "'Manrope', sans-serif",
              }}
            >
              AI Score
            </span>
          ) : null}
          {token.deep_researched ? (
            <span
              style={{
                color: SHELL.green,
                background: 'rgba(54,239,182,0.12)',
                borderRadius: 4,
                padding: '2px 6px',
                fontSize: 9,
                fontWeight: 800,
                letterSpacing: '0.1em',
                textTransform: 'uppercase',
                fontFamily: "'Manrope', sans-serif",
              }}
            >
              Deep Research
            </span>
          ) : null}
        </div>
        <div style={{ fontSize: 11, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
          {metric.volumeValue} · {metric.flowValue} · TX {formatNumber(token.total_tx, 0)} · {metric.hint}
        </div>
        <div style={{ marginTop: 4, fontSize: 10, color: SHELL.blue, fontFamily: "'Space Grotesk', sans-serif", lineHeight: 1.45 }}>
          Watching for: {token.watching_for}
        </div>
        <div style={{ marginTop: 2, fontSize: 11, color: SHELL.muted, fontFamily: "'Roboto Mono', monospace" }}>
          {shortAddress(token.contract_address)}
        </div>
      </div>
      <button
        type="button"
        onClick={() => onCopy(token.contract_address)}
        style={{
          border: `1px solid ${SHELL.border}`,
          background: 'rgba(255,255,255,0.04)',
          color: SHELL.muted,
          width: 34,
          height: 34,
          borderRadius: 10,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <FaCopy size={12} />
      </button>
    </div>
  );
}

function ControlChip({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        flexShrink: 0,
        background: active ? 'rgba(111,141,255,0.14)' : 'rgba(255,255,255,0.03)',
        border: active ? '1px solid rgba(111,141,255,0.22)' : `1px solid ${SHELL.border}`,
        color: active ? SHELL.blue : SHELL.muted,
        borderRadius: 999,
        padding: '8px 12px',
        fontSize: 10,
        fontWeight: 800,
        letterSpacing: '0.12em',
        textTransform: 'uppercase',
        fontFamily: "'Manrope', sans-serif",
      }}
    >
      {label}
    </button>
  );
}

interface HomeLandingClientProps {
  initialFeed: TokenListResponse;
}

export default function HomeLandingClient({ initialFeed }: HomeLandingClientProps) {
  const router = useRouter();
  const [query, setQuery] = useState('');
  const [view, setView] = useState<FeedView>('latest');
  const [discoverFilter, setDiscoverFilter] = useState<DiscoverFilter>('all');
  const [discoverSort, setDiscoverSort] = useState<DiscoverSort>('feed');
  const [page, setPage] = useState(1);
  const [feed, setFeed] = useState<TokenListResponse>(initialFeed);
  const [opsSummary, setOpsSummary] = useState<InvestigationOpsSummaryResponse | null>(null);
  const [priorityRuns, setPriorityRuns] = useState<InvestigationRunSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedAddress, setCopiedAddress] = useState<string | null>(null);

  const totalPages = Math.max(1, Math.ceil(feed.total / PAGE_SIZE));

  useEffect(() => {
    setPage(1);
  }, [view, discoverFilter, discoverSort]);

  useEffect(() => {
    let active = true;
    const selected = FEED_VIEWS[view];

    const load = async () => {
      if (view === 'latest' && page === 1 && discoverFilter === 'all' && discoverSort === 'feed') {
        setFeed(initialFeed);
        setError(null);
        return;
      }

      setLoading(true);
      try {
        const response = await api.tokens.list({
          limit: PAGE_SIZE,
          offset: (page - 1) * PAGE_SIZE,
          sort: discoverSort === 'tx' ? 'tx' : selected.sort,
          ...(selected.windowHours ? { window_hours: selected.windowHours } : {}),
          ...(discoverFilter === 'ai_scored' ? { ai_scored: true } : {}),
          ...(discoverFilter === 'deep_research' ? { deep_research: true } : {}),
        });

        if (!active) return;
        setFeed(response);
        setError(null);
      } catch (err) {
        if (!active) return;
        setError(err instanceof Error ? err.message : 'Failed to load token feed.');
      } finally {
        if (active) setLoading(false);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, [initialFeed, page, view, discoverFilter, discoverSort]);

  useEffect(() => {
    let active = true;

    const loadSummary = async () => {
      try {
        const [summary, escalatedRuns, watchingRuns] = await Promise.all([
          api.investigations.opsSummary(),
          api.investigations.runs({ limit: 2, status: 'escalated' }),
          api.investigations.runs({ limit: 2, status: 'watching' }),
        ]);

        if (!active) return;

        const merged = [...escalatedRuns.data, ...watchingRuns.data]
          .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
          .slice(0, 3);

        setOpsSummary(summary);
        setPriorityRuns(merged);
      } catch {
        if (!active) return;
        setOpsSummary(null);
        setPriorityRuns([]);
      }
    };

    void loadSummary();
    const id = window.setInterval(loadSummary, 30_000);

    return () => {
      active = false;
      window.clearInterval(id);
    };
  }, []);

  useEffect(() => {
    if (!copiedAddress) return;
    const id = window.setTimeout(() => setCopiedAddress(null), 1400);
    return () => window.clearTimeout(id);
  }, [copiedAddress]);

  const topRun = priorityRuns[0] ?? null;
  const topToken = feed.data[0] ?? null;
  const heading = useMemo(() => (discoverSort === 'tx' ? 'Top Transactions' : FEED_VIEWS[view].title), [discoverSort, view]);

  const briefingText = useMemo(() => {
    if (topRun) {
      return `MIA has ${priorityRuns.length} active attention lane${priorityRuns.length > 1 ? 's' : ''}. The most urgent run is ${topRun.status} on ${shortAddress(topRun.token_address)} and last changed ${timeAgo(topRun.updated_at)}.`;
    }
    if (topToken) {
      return `${topToken.symbol ?? 'A token'} is at the top of the live feed right now. Open it to inspect the evidence, then decide if it belongs in runs or watch.`;
    }
    return 'MIA is scanning the Four.Meme universe and preparing the next investigation surface.';
  }, [priorityRuns.length, topRun, topToken]);

  const submitSearch = () => {
    const value = query.trim();
    if (!value) return;
    router.push(`/mia?q=${encodeURIComponent(value)}`);
  };

  return (
    <>
      <ObsidianNav />
      <main
        className="obsidian-page min-h-screen px-3 pb-16 pt-4"
        style={{
          background:
            'radial-gradient(circle at top, rgba(111,141,255,0.14), transparent 32%), radial-gradient(circle at 20% 20%, rgba(54,239,182,0.08), transparent 28%), #08101d',
          color: SHELL.ink,
        }}
      >
        <div className="mx-auto w-full max-w-[430px] lg:hidden" data-testid="discover-page-heading">
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              background: SHELL.panelSoft,
              borderRadius: 14,
              border: `1px solid rgba(111,141,255,0.2)`,
              padding: '12px 16px',
              marginBottom: 16,
            }}
          >
            <FaMagnifyingGlass size={16} color={SHELL.blue} />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') submitSearch();
              }}
              placeholder="Enter ticker or 0x address..."
              style={{
                flex: 1,
                background: 'none',
                border: 'none',
                outline: 'none',
                color: SHELL.ink,
                fontSize: 14,
                fontFamily: "'Space Grotesk', sans-serif",
              }}
            />
            {query.trim() ? (
              <button
                type="button"
                onClick={submitSearch}
                style={{
                  background: SHELL.blue,
                  border: 'none',
                  borderRadius: 8,
                  color: '#081736',
                  padding: '6px 12px',
                  fontSize: 11,
                  fontWeight: 800,
                  cursor: 'pointer',
                  fontFamily: "'Manrope', sans-serif",
                  letterSpacing: '0.1em',
                  textTransform: 'uppercase',
                }}
              >
                Go
              </button>
            ) : null}
          </div>

          <div
            style={{
              marginBottom: 16,
              background: 'linear-gradient(135deg, rgba(111,141,255,0.18) 0%, rgba(12,16,24,0.98) 60%)',
              borderRadius: 16,
              border: '1px solid rgba(111,141,255,0.22)',
              padding: '20px 16px',
              position: 'relative',
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                position: 'absolute',
                top: 0,
                right: 0,
                width: 120,
                height: 120,
                background: 'radial-gradient(circle, rgba(54,239,182,0.12) 0%, transparent 70%)',
              }}
            />
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
              <LiveDot />
              <span
                style={{
                  fontSize: 10,
                  color: SHELL.muted,
                  fontFamily: "'Manrope', sans-serif",
                  letterSpacing: '0.14em',
                  textTransform: 'uppercase',
                  fontWeight: 700,
                }}
              >
                MIA Intelligence Briefing
              </span>
            </div>
            <p
              style={{
                fontSize: 14,
                lineHeight: 1.6,
                color: SHELL.ink,
                fontFamily: "'Space Grotesk', sans-serif",
                margin: 0,
                marginBottom: 14,
                minHeight: 66,
              }}
            >
              {briefingText}
            </p>
            <div style={{ display: 'flex', gap: 8 }}>
              <Link
                href="/mia"
                data-testid="discover-open-investigation"
                style={{
                  flex: 1,
                  padding: '10px 0',
                  borderRadius: 10,
                  background: SHELL.blue,
                  color: '#081736',
                  fontSize: 11,
                  fontWeight: 800,
                  letterSpacing: '0.12em',
                  textTransform: 'uppercase',
                  textAlign: 'center',
                  fontFamily: "'Manrope', sans-serif",
                }}
              >
                Investigate
              </Link>
              <Link
                href="/mia/runs"
                style={{
                  flex: 1,
                  padding: '10px 0',
                  borderRadius: 10,
                  background: 'rgba(54,239,182,0.15)',
                  color: SHELL.green,
                  border: '1px solid rgba(54,239,182,0.25)',
                  fontSize: 11,
                  fontWeight: 800,
                  letterSpacing: '0.12em',
                  textTransform: 'uppercase',
                  textAlign: 'center',
                  fontFamily: "'Manrope', sans-serif",
                }}
              >
                View Runs
              </Link>
            </div>
          </div>

          <div style={{ display: 'flex', gap: 10, marginBottom: 16 }}>
            <StatCard label="Live Tokens" value={String(feed.total)} color={SHELL.green} />
            <StatCard
              label="Active Runs"
              value={String((opsSummary?.runs.queued ?? 0) + (opsSummary?.runs.running ?? 0) + (opsSummary?.runs.watching ?? 0) + (opsSummary?.runs.escalated ?? 0))}
              color={SHELL.blue}
            />
            <StatCard label="Missions" value={String(opsSummary?.missions.active ?? 0)} color={SHELL.yellow} />
          </div>

          <SectionLabel title="Quick Access" />
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10, margin: '0 0 16px' }}>
            <QuickActionCard href="/mia" icon={<FaMagnifyingGlass />} title="Investigate" sub="Open token read" color={SHELL.blue} />
            <QuickActionCard href="/alpha" icon={<FaBolt />} title="Alpha Feed" sub="Top signals" color={SHELL.green} />
            <QuickActionCard href="/tokens" icon={<FaRadio />} title="Live Feed" sub="New deployments" color="#a8b8ff" />
            <QuickActionCard href="/mia/watchlist" icon={<FaBell />} title="Watchlist" sub="My monitoring" color={SHELL.red} />
            <QuickActionCard href="/mia/runs" icon={<FaWaveSquare />} title="Runs" sub="Live investigations" color={SHELL.yellow} />
            <QuickActionCard href="/backtesting" icon={<FaChartLine />} title="Proof Lab" sub="Replay context" color={SHELL.muted} />
          </div>

          <SectionLabel title="Focus Now" action={topRun ? 'Open Run' : 'Open Feed'} href={topRun ? '/mia/runs' : '/tokens'} />
          <div
            style={{
              marginBottom: 16,
              background: topRun
                ? 'linear-gradient(135deg, rgba(111,141,255,0.14), rgba(12,16,24,0.97))'
                : SHELL.panel,
              borderRadius: 14,
              border: topRun ? '1px solid rgba(111,141,255,0.2)' : `1px solid ${SHELL.border}`,
              padding: '14px 16px',
            }}
          >
            {topRun ? (
              <>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 10 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span
                      style={{
                        color: toneForRunStatus(topRun.status).color,
                        background: toneForRunStatus(topRun.status).bg,
                        borderRadius: 6,
                        padding: '3px 8px',
                        fontSize: 10,
                        fontWeight: 800,
                        letterSpacing: '0.1em',
                        textTransform: 'uppercase',
                        fontFamily: "'Manrope', sans-serif",
                      }}
                    >
                      {topRun.status}
                    </span>
                    <span style={{ fontSize: 10, color: SHELL.muted, fontFamily: "'Space Grotesk', sans-serif" }}>
                      {timeAgo(topRun.updated_at)}
                    </span>
                  </div>
                  <Link
                    href={`/mia?q=${encodeURIComponent(topRun.token_address)}`}
                    data-testid="discover-priority-open-investigation"
                    style={{
                      color: SHELL.blue,
                      fontSize: 10,
                      fontWeight: 800,
                      letterSpacing: '0.12em',
                      textTransform: 'uppercase',
                      fontFamily: "'Manrope', sans-serif",
                    }}
                  >
                    Open
                  </Link>
                </div>
                <div style={{ marginTop: 12, display: 'flex', alignItems: 'center', gap: 12 }}>
                  <ScoreRing score={70} size={56} stroke={4} />
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div style={{ fontSize: 16, fontWeight: 900, fontFamily: "'Manrope', sans-serif", color: SHELL.ink }}>
                      {shortAddress(topRun.token_address, 8, 6)}
                    </div>
                    <p style={{ margin: '4px 0 0', fontSize: 12, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif", lineHeight: 1.5 }}>
                      {topRun.summary ?? 'Priority run is ready to reopen in the investigation workspace.'}
                    </p>
                  </div>
                </div>
              </>
            ) : topToken ? (
              <Link href={`/mia?q=${encodeURIComponent(topToken.contract_address)}`} style={{ display: 'block' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <ScoreRing score={displayScore(topToken)} size={56} stroke={4} />
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4, flexWrap: 'wrap' }}>
                      <span style={{ fontSize: 16, fontWeight: 900, fontFamily: "'Manrope', sans-serif", color: SHELL.ink }}>
                        {topToken.symbol ?? topToken.name ?? shortAddress(topToken.contract_address)}
                      </span>
                      <span
                        style={{
                          fontSize: 9,
                          color: SHELL.green,
                          background: 'rgba(54,239,182,0.1)',
                          border: '1px solid rgba(54,239,182,0.2)',
                          borderRadius: 4,
                          padding: '2px 6px',
                          fontFamily: "'Manrope', sans-serif",
                          fontWeight: 800,
                          letterSpacing: '0.1em',
                          textTransform: 'uppercase',
                        }}
                      >
                        Featured
                      </span>
                    </div>
                    <p style={{ margin: 0, fontSize: 12, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif", lineHeight: 1.5 }}>
                      {feedMetric(topToken, view).volumeValue} · {feedMetric(topToken, view).flowValue} · {timeAgo(topToken.deployed_at)}
                    </p>
                  </div>
                </div>
              </Link>
            ) : (
              <div style={{ fontSize: 12, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                No priority surface is available yet.
              </div>
            )}
          </div>

          <SectionLabel title={heading} action="Open Full Feed" href="/tokens" />
          <div
            style={{
              marginBottom: 16,
              background: SHELL.panel,
              borderRadius: 14,
              border: `1px solid ${SHELL.border}`,
              overflow: 'hidden',
            }}
          >
            <div style={{ display: 'flex', gap: 6, padding: '12px 16px', borderBottom: '1px solid rgba(255,255,255,0.04)', overflowX: 'auto' }}>
              {(Object.keys(FEED_VIEWS) as FeedView[]).map((item) => {
                const active = item === view;
                return (
                  <button
                    key={item}
                    type="button"
                    onClick={() => setView(item)}
                    style={{
                      flexShrink: 0,
                      background: active ? 'rgba(111,141,255,0.14)' : 'rgba(255,255,255,0.03)',
                      border: active ? '1px solid rgba(111,141,255,0.22)' : `1px solid ${SHELL.border}`,
                      color: active ? SHELL.blue : SHELL.muted,
                      borderRadius: 999,
                      padding: '7px 12px',
                      fontSize: 10,
                      fontWeight: 800,
                      letterSpacing: '0.12em',
                      textTransform: 'uppercase',
                      fontFamily: "'Manrope', sans-serif",
                    }}
                  >
                    {FEED_VIEWS[item].label}
                  </button>
                );
              })}
            </div>
            <div style={{ display: 'flex', gap: 6, padding: '12px 16px 8px', overflowX: 'auto' }}>
              <ControlChip active={discoverFilter === 'all'} label="All" onClick={() => setDiscoverFilter('all')} />
              <ControlChip active={discoverFilter === 'ai_scored'} label="AI Scored" onClick={() => setDiscoverFilter('ai_scored')} />
              <ControlChip active={discoverFilter === 'deep_research'} label="Deep Research" onClick={() => setDiscoverFilter('deep_research')} />
            </div>
            <div style={{ display: 'flex', gap: 6, padding: '0 16px 12px', overflowX: 'auto', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
              <ControlChip active={discoverSort === 'feed'} label="Feed Sort" onClick={() => setDiscoverSort('feed')} />
              <ControlChip active={discoverSort === 'tx'} label="Top TX" onClick={() => setDiscoverSort('tx')} />
            </div>

            {feed.data.length === 0 && !loading ? (
              <div style={{ padding: '18px 16px', color: SHELL.body, fontSize: 12, fontFamily: "'Space Grotesk', sans-serif" }}>
                No tokens are available in this feed yet.
              </div>
            ) : (
              feed.data.map((token) => (
                <Link key={`${view}:${token.contract_address}`} href={`/mia?q=${encodeURIComponent(token.contract_address)}`} style={{ display: 'block' }}>
                  <FeedRow
                    token={token}
                    view={view}
                    onCopy={async (address) => {
                      await navigator.clipboard.writeText(address);
                      setCopiedAddress(address);
                    }}
                  />
                </Link>
              ))
            )}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 10, marginBottom: 8 }}>
            <div style={{ fontSize: 11, color: SHELL.muted, fontFamily: "'Space Grotesk', sans-serif" }}>
              Showing {feed.data.length === 0 ? 0 : (page - 1) * PAGE_SIZE + 1}-{Math.min(page * PAGE_SIZE, feed.total)} of {feed.total.toLocaleString()}
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <button
                type="button"
                onClick={() => setPage((current) => Math.max(1, current - 1))}
                disabled={page <= 1}
                style={{
                  border: `1px solid ${SHELL.border}`,
                  background: 'rgba(255,255,255,0.04)',
                  color: SHELL.ink,
                  width: 36,
                  height: 36,
                  borderRadius: 10,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  opacity: page <= 1 ? 0.4 : 1,
                }}
              >
                <FaChevronLeft size={12} />
              </button>
              <div
                style={{
                  border: `1px solid ${SHELL.border}`,
                  background: 'rgba(255,255,255,0.04)',
                  color: SHELL.body,
                  borderRadius: 10,
                  padding: '10px 12px',
                  fontSize: 10,
                  fontWeight: 800,
                  letterSpacing: '0.1em',
                  textTransform: 'uppercase',
                  fontFamily: "'Manrope', sans-serif",
                }}
              >
                Page {page}/{totalPages}
              </div>
              <button
                type="button"
                onClick={() => setPage((current) => Math.min(totalPages, current + 1))}
                disabled={page >= totalPages}
                style={{
                  border: `1px solid ${SHELL.border}`,
                  background: 'rgba(255,255,255,0.04)',
                  color: SHELL.ink,
                  width: 36,
                  height: 36,
                  borderRadius: 10,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  opacity: page >= totalPages ? 0.4 : 1,
                }}
              >
                <FaChevronRight size={12} />
              </button>
            </div>
          </div>

          {copiedAddress ? (
            <div style={{ marginTop: 8, fontSize: 11, color: SHELL.green, fontFamily: "'Space Grotesk', sans-serif" }}>
              Copied {shortAddress(copiedAddress)}.
            </div>
          ) : null}

          {loading ? (
            <div style={{ marginTop: 8, fontSize: 11, color: SHELL.muted, fontFamily: "'Space Grotesk', sans-serif" }}>
              Refreshing feed...
            </div>
          ) : null}

          {error ? (
            <div style={{ marginTop: 8, fontSize: 11, color: SHELL.red, fontFamily: "'Space Grotesk', sans-serif" }}>
              {error}
            </div>
          ) : null}

          <div
            style={{
              marginTop: 20,
              borderRadius: 14,
              border: `1px solid ${SHELL.border}`,
              background: 'rgba(255,255,255,0.03)',
              padding: '14px 16px',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12 }}>
              <div>
                <div
                  style={{
                    fontSize: 10,
                    color: SHELL.muted,
                    fontWeight: 800,
                    letterSpacing: '0.16em',
                    textTransform: 'uppercase',
                    fontFamily: "'Manrope', sans-serif",
                  }}
                >
                  Deep entry
                </div>
                <div style={{ marginTop: 4, fontSize: 13, color: SHELL.ink, fontFamily: "'Manrope', sans-serif", fontWeight: 800 }}>
                  Need the full token console?
                </div>
                <div style={{ marginTop: 2, fontSize: 11, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                  Jump straight into the investigation workspace when a token deserves the full read.
                </div>
              </div>
              <Link
                href={topToken ? `/mia?q=${encodeURIComponent(topToken.contract_address)}` : '/mia'}
                style={{
                  width: 42,
                  height: 42,
                  borderRadius: 12,
                  background: SHELL.blue,
                  color: '#081736',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexShrink: 0,
                }}
              >
                <FaArrowRight size={14} />
              </Link>
            </div>
          </div>
        </div>

        <div className="mx-auto hidden w-full max-w-[1420px] gap-6 lg:grid lg:grid-cols-[240px_minmax(0,1fr)_320px] lg:px-2">
          <aside className="sticky top-24 h-fit space-y-4">
            <div
              style={{
                background: SHELL.panelSoft,
                border: `1px solid ${SHELL.border}`,
                borderRadius: 22,
                padding: 20,
              }}
            >
              <div style={{ fontSize: 12, color: SHELL.muted, letterSpacing: '0.18em', textTransform: 'uppercase', fontWeight: 800, fontFamily: "'Manrope', sans-serif" }}>
                MIA Surface
              </div>
              <div style={{ marginTop: 8, fontSize: 28, lineHeight: 1.05, fontWeight: 900, color: SHELL.ink, fontFamily: "'Manrope', sans-serif" }}>
                Discover what matters now.
              </div>
              <p style={{ marginTop: 10, fontSize: 13, lineHeight: 1.7, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                Fast entry into live candidates, urgent runs, and the next token worth opening in MIA.
              </p>
            </div>

            <div
              style={{
                background: SHELL.panel,
                border: `1px solid ${SHELL.border}`,
                borderRadius: 22,
                padding: 18,
              }}
            >
              <div style={{ fontSize: 10, color: SHELL.muted, letterSpacing: '0.18em', textTransform: 'uppercase', fontWeight: 800, fontFamily: "'Manrope', sans-serif", marginBottom: 12 }}>
                Quick Access
              </div>
              <div className="grid gap-3">
                <QuickActionCard href="/mia" icon={<FaMagnifyingGlass />} title="Investigate" sub="Open token read" color={SHELL.blue} />
                <QuickActionCard href="/mia/runs" icon={<FaWaveSquare />} title="Runs" sub="Live investigations" color={SHELL.yellow} />
                <QuickActionCard href="/mia/watchlist" icon={<FaBell />} title="Watch" sub="Persistent monitoring" color={SHELL.red} />
                <QuickActionCard href="/mia/missions" icon={<FaBolt />} title="Missions" sub="Operator objectives" color={SHELL.green} />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <StatCard label="Live Tokens" value={String(feed.total)} color={SHELL.green} />
              <StatCard
                label="Runs"
                value={String((opsSummary?.runs.queued ?? 0) + (opsSummary?.runs.running ?? 0) + (opsSummary?.runs.watching ?? 0) + (opsSummary?.runs.escalated ?? 0))}
                color={SHELL.blue}
              />
              <StatCard label="Watch" value={String(opsSummary?.watchlist_items ?? 0)} color={SHELL.red} />
              <StatCard label="Missions" value={String(opsSummary?.missions.active ?? 0)} color={SHELL.yellow} />
            </div>
          </aside>

          <section className="min-w-0">
            <div
              data-testid="discover-page-heading"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 14,
                background: SHELL.panelSoft,
                borderRadius: 22,
                border: '1px solid rgba(111,141,255,0.16)',
                padding: '16px 18px',
                marginBottom: 18,
              }}
            >
              <FaMagnifyingGlass size={18} color={SHELL.blue} />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') submitSearch();
                }}
                placeholder="Search ticker or contract address..."
                style={{
                  flex: 1,
                  background: 'none',
                  border: 'none',
                  outline: 'none',
                  color: SHELL.ink,
                  fontSize: 15,
                  fontFamily: "'Space Grotesk', sans-serif",
                }}
              />
              <Link
                href="/mia"
                style={{
                  padding: '12px 18px',
                  borderRadius: 14,
                  background: 'rgba(255,255,255,0.04)',
                  color: SHELL.body,
                  border: `1px solid ${SHELL.border}`,
                  fontSize: 11,
                  fontWeight: 800,
                  letterSpacing: '0.14em',
                  textTransform: 'uppercase',
                  fontFamily: "'Manrope', sans-serif",
                  whiteSpace: 'nowrap',
                }}
              >
                Open Workspace
              </Link>
              <button
                type="button"
                onClick={submitSearch}
                style={{
                  padding: '12px 20px',
                  borderRadius: 14,
                  background: SHELL.blue,
                  color: '#081736',
                  border: 'none',
                  fontSize: 11,
                  fontWeight: 800,
                  letterSpacing: '0.14em',
                  textTransform: 'uppercase',
                  fontFamily: "'Manrope', sans-serif",
                  cursor: 'pointer',
                  whiteSpace: 'nowrap',
                }}
              >
                Investigate
              </button>
            </div>

            <div
              style={{
                marginBottom: 18,
                background: 'linear-gradient(135deg, rgba(111,141,255,0.18) 0%, rgba(12,16,24,0.98) 60%)',
                borderRadius: 26,
                border: '1px solid rgba(111,141,255,0.2)',
                padding: 24,
                position: 'relative',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  position: 'absolute',
                  top: -12,
                  right: -12,
                  width: 180,
                  height: 180,
                  background: 'radial-gradient(circle, rgba(54,239,182,0.12) 0%, transparent 70%)',
                }}
              />
              <div className="mb-3 flex items-center justify-between gap-4">
                <div className="flex items-center gap-2">
                  <LiveDot />
                  <span style={{ fontSize: 10, color: SHELL.muted, letterSpacing: '0.16em', textTransform: 'uppercase', fontWeight: 800, fontFamily: "'Manrope', sans-serif" }}>
                    MIA Intelligence Briefing
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  {(Object.keys(FEED_VIEWS) as FeedView[]).map((item) => {
                    const active = item === view;
                    return (
                      <button
                        key={item}
                        type="button"
                        onClick={() => setView(item)}
                        style={{
                          background: active ? 'rgba(111,141,255,0.14)' : 'rgba(255,255,255,0.03)',
                          border: active ? '1px solid rgba(111,141,255,0.22)' : `1px solid ${SHELL.border}`,
                          color: active ? SHELL.blue : SHELL.muted,
                          borderRadius: 999,
                          padding: '8px 13px',
                          fontSize: 10,
                          fontWeight: 800,
                          letterSpacing: '0.12em',
                          textTransform: 'uppercase',
                          fontFamily: "'Manrope', sans-serif",
                        }}
                      >
                        {FEED_VIEWS[item].label}
                      </button>
                    );
                  })}
                </div>
              </div>
              <div className="grid gap-5 xl:grid-cols-[minmax(0,1.15fr)_320px]">
                <div>
                  <div style={{ fontSize: 34, lineHeight: 1.05, fontWeight: 900, color: SHELL.ink, fontFamily: "'Manrope', sans-serif", maxWidth: 620 }}>
                    Open the next live token before the feed gets noisy.
                  </div>
                  <p style={{ marginTop: 12, fontSize: 15, lineHeight: 1.8, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                    {briefingText}
                  </p>
                  <div className="mt-5 flex flex-wrap gap-2">
                    <ControlChip active={discoverFilter === 'all'} label="All" onClick={() => setDiscoverFilter('all')} />
                    <ControlChip active={discoverFilter === 'ai_scored'} label="AI Scored" onClick={() => setDiscoverFilter('ai_scored')} />
                    <ControlChip active={discoverFilter === 'deep_research'} label="Deep Research" onClick={() => setDiscoverFilter('deep_research')} />
                    <ControlChip active={discoverSort === 'feed'} label="Feed Sort" onClick={() => setDiscoverSort('feed')} />
                    <ControlChip active={discoverSort === 'tx'} label="Top TX" onClick={() => setDiscoverSort('tx')} />
                  </div>
                  <div className="mt-5 flex gap-3">
                    <Link
                      href="/mia"
                      data-testid="discover-open-investigation"
                      style={{
                        padding: '12px 18px',
                        borderRadius: 14,
                        background: SHELL.blue,
                        color: '#081736',
                        fontSize: 11,
                        fontWeight: 800,
                        letterSpacing: '0.12em',
                        textTransform: 'uppercase',
                        textAlign: 'center',
                        fontFamily: "'Manrope', sans-serif",
                      }}
                    >
                      Start Investigation
                    </Link>
                    <Link
                      href="/mia/runs"
                      style={{
                        padding: '12px 18px',
                        borderRadius: 14,
                        background: 'rgba(54,239,182,0.15)',
                        color: SHELL.green,
                        border: '1px solid rgba(54,239,182,0.25)',
                        fontSize: 11,
                        fontWeight: 800,
                        letterSpacing: '0.12em',
                        textTransform: 'uppercase',
                        textAlign: 'center',
                        fontFamily: "'Manrope', sans-serif",
                      }}
                    >
                      Open Runs
                    </Link>
                  </div>
                </div>

                <div
                  style={{
                    background: 'rgba(8,16,29,0.64)',
                    border: `1px solid ${SHELL.border}`,
                    borderRadius: 20,
                    padding: 18,
                  }}
                >
                  <div style={{ fontSize: 10, color: SHELL.muted, letterSpacing: '0.16em', textTransform: 'uppercase', fontWeight: 800, fontFamily: "'Manrope', sans-serif", marginBottom: 10 }}>
                    Focus Now
                  </div>
                  {topRun ? (
                    <>
                      <div className="mb-3 flex items-center justify-between gap-3">
                        <span
                          style={{
                            color: toneForRunStatus(topRun.status).color,
                            background: toneForRunStatus(topRun.status).bg,
                            borderRadius: 999,
                            padding: '4px 10px',
                            fontSize: 10,
                            fontWeight: 800,
                            letterSpacing: '0.1em',
                            textTransform: 'uppercase',
                            fontFamily: "'Manrope', sans-serif",
                          }}
                        >
                          {topRun.status}
                        </span>
                        <span style={{ fontSize: 11, color: SHELL.muted, fontFamily: "'Space Grotesk', sans-serif" }}>{timeAgo(topRun.updated_at)}</span>
                      </div>
                      <div className="mb-3 flex items-center gap-3">
                        <ScoreRing score={70} size={62} stroke={4} />
                        <div>
                          <div style={{ fontSize: 17, fontWeight: 900, color: SHELL.ink, fontFamily: "'Manrope', sans-serif" }}>
                            {shortAddress(topRun.token_address, 8, 6)}
                          </div>
                          <div style={{ marginTop: 4, fontSize: 12, color: SHELL.body, lineHeight: 1.6, fontFamily: "'Space Grotesk', sans-serif" }}>
                            {topRun.summary ?? 'Priority run is ready to reopen in the investigation workspace.'}
                          </div>
                        </div>
                      </div>
                      <Link
                        href={`/mia?q=${encodeURIComponent(topRun.token_address)}`}
                        data-testid="discover-priority-open-investigation"
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: 8,
                          padding: '10px 14px',
                          borderRadius: 12,
                          background: 'rgba(255,255,255,0.05)',
                          border: `1px solid ${SHELL.border}`,
                          color: SHELL.ink,
                          fontSize: 11,
                          fontWeight: 800,
                          letterSpacing: '0.12em',
                          textTransform: 'uppercase',
                          fontFamily: "'Manrope', sans-serif",
                        }}
                      >
                        Open Priority
                        <FaArrowRight size={12} />
                      </Link>
                    </>
                  ) : topToken ? (
                    <Link href={`/mia?q=${encodeURIComponent(topToken.contract_address)}`} style={{ display: 'block' }}>
                      <div className="flex items-center gap-3">
                        <ScoreRing score={displayScore(topToken)} size={62} stroke={4} />
                        <div>
                          <div style={{ fontSize: 17, fontWeight: 900, color: SHELL.ink, fontFamily: "'Manrope', sans-serif" }}>
                            {topToken.symbol ?? topToken.name ?? shortAddress(topToken.contract_address)}
                          </div>
                          <div style={{ marginTop: 4, fontSize: 12, color: SHELL.body, lineHeight: 1.6, fontFamily: "'Space Grotesk', sans-serif" }}>
                            {feedMetric(topToken, view).volumeValue} · {feedMetric(topToken, view).flowValue}
                          </div>
                        </div>
                      </div>
                    </Link>
                  ) : (
                    <div style={{ fontSize: 12, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                      No featured token is available yet.
                    </div>
                  )}
                </div>
              </div>
            </div>

            <div
              style={{
                background: SHELL.panel,
                borderRadius: 24,
                border: `1px solid ${SHELL.border}`,
                overflow: 'hidden',
              }}
            >
              <div className="flex items-center justify-between gap-4 border-b px-5 py-4" style={{ borderColor: 'rgba(255,255,255,0.04)' }}>
                <div>
                  <div style={{ fontSize: 11, color: SHELL.muted, letterSpacing: '0.16em', textTransform: 'uppercase', fontWeight: 800, fontFamily: "'Manrope', sans-serif" }}>
                    {heading}
                  </div>
                  <div style={{ marginTop: 4, fontSize: 20, fontWeight: 900, color: SHELL.ink, fontFamily: "'Manrope', sans-serif" }}>
                    Live token candidates
                  </div>
                </div>
                <Link
                  href="/tokens"
                  style={{
                    padding: '11px 15px',
                    borderRadius: 12,
                    background: 'rgba(255,255,255,0.04)',
                    border: `1px solid ${SHELL.border}`,
                    color: SHELL.body,
                    fontSize: 11,
                    fontWeight: 800,
                    letterSpacing: '0.12em',
                    textTransform: 'uppercase',
                    fontFamily: "'Manrope', sans-serif",
                  }}
                >
                  Open Full Feed
                </Link>
              </div>
              <div className="flex flex-wrap gap-2 border-b px-5 py-4" style={{ borderColor: 'rgba(255,255,255,0.04)' }}>
                <ControlChip active={discoverFilter === 'all'} label="All" onClick={() => setDiscoverFilter('all')} />
                <ControlChip active={discoverFilter === 'ai_scored'} label="AI Scored" onClick={() => setDiscoverFilter('ai_scored')} />
                <ControlChip active={discoverFilter === 'deep_research'} label="Deep Research" onClick={() => setDiscoverFilter('deep_research')} />
                <ControlChip active={discoverSort === 'feed'} label="Feed Sort" onClick={() => setDiscoverSort('feed')} />
                <ControlChip active={discoverSort === 'tx'} label="Top TX" onClick={() => setDiscoverSort('tx')} />
              </div>

              {feed.data.length === 0 && !loading ? (
                <div style={{ padding: '18px 20px', color: SHELL.body, fontSize: 12, fontFamily: "'Space Grotesk', sans-serif" }}>
                  No tokens are available in this feed yet.
                </div>
              ) : (
                feed.data.map((token) => (
                  <Link key={`${view}:${token.contract_address}`} href={`/mia?q=${encodeURIComponent(token.contract_address)}`} style={{ display: 'block' }}>
                    <FeedRow
                      token={token}
                      view={view}
                      onCopy={async (address) => {
                        await navigator.clipboard.writeText(address);
                        setCopiedAddress(address);
                      }}
                    />
                  </Link>
                ))
              )}
            </div>

            <div className="mt-4 flex items-center justify-between gap-4">
              <div style={{ fontSize: 12, color: SHELL.muted, fontFamily: "'Space Grotesk', sans-serif" }}>
                Showing {feed.data.length === 0 ? 0 : (page - 1) * PAGE_SIZE + 1}-{Math.min(page * PAGE_SIZE, feed.total)} of {feed.total.toLocaleString()}
              </div>
              <div className="flex items-center gap-3">
                <button
                  type="button"
                  onClick={() => setPage((current) => Math.max(1, current - 1))}
                  disabled={page <= 1}
                  style={{
                    border: `1px solid ${SHELL.border}`,
                    background: 'rgba(255,255,255,0.04)',
                    color: SHELL.ink,
                    width: 42,
                    height: 42,
                    borderRadius: 12,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    opacity: page <= 1 ? 0.4 : 1,
                  }}
                >
                  <FaChevronLeft size={13} />
                </button>
                <div
                  style={{
                    border: `1px solid ${SHELL.border}`,
                    background: 'rgba(255,255,255,0.04)',
                    color: SHELL.body,
                    borderRadius: 12,
                    padding: '11px 14px',
                    fontSize: 11,
                    fontWeight: 800,
                    letterSpacing: '0.1em',
                    textTransform: 'uppercase',
                    fontFamily: "'Manrope', sans-serif",
                  }}
                >
                  Page {page}/{totalPages}
                </div>
                <button
                  type="button"
                  onClick={() => setPage((current) => Math.min(totalPages, current + 1))}
                  disabled={page >= totalPages}
                  style={{
                    border: `1px solid ${SHELL.border}`,
                    background: 'rgba(255,255,255,0.04)',
                    color: SHELL.ink,
                    width: 42,
                    height: 42,
                    borderRadius: 12,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    opacity: page >= totalPages ? 0.4 : 1,
                  }}
                >
                  <FaChevronRight size={13} />
                </button>
              </div>
            </div>

            {copiedAddress ? (
              <div style={{ marginTop: 8, fontSize: 11, color: SHELL.green, fontFamily: "'Space Grotesk', sans-serif" }}>
                Copied {shortAddress(copiedAddress)}.
              </div>
            ) : null}

            {loading ? (
              <div style={{ marginTop: 8, fontSize: 11, color: SHELL.muted, fontFamily: "'Space Grotesk', sans-serif" }}>
                Refreshing feed...
              </div>
            ) : null}

            {error ? (
              <div style={{ marginTop: 8, fontSize: 11, color: SHELL.red, fontFamily: "'Space Grotesk', sans-serif" }}>
                {error}
              </div>
            ) : null}
          </section>

          <aside className="sticky top-24 h-fit space-y-4">
            <div
              style={{
                background: SHELL.panel,
                border: `1px solid ${SHELL.border}`,
                borderRadius: 22,
                padding: 18,
              }}
            >
              <div style={{ fontSize: 10, color: SHELL.muted, letterSpacing: '0.18em', textTransform: 'uppercase', fontWeight: 800, fontFamily: "'Manrope', sans-serif", marginBottom: 12 }}>
                Priority Lanes
              </div>
              <div className="space-y-3">
                {priorityRuns.length > 0 ? (
                  priorityRuns.map((run) => (
                    <Link
                      key={run.run_id}
                      href={`/mia?q=${encodeURIComponent(run.token_address)}`}
                      style={{
                        display: 'block',
                        borderRadius: 16,
                        padding: 14,
                        background: 'rgba(255,255,255,0.03)',
                        border: `1px solid ${SHELL.border}`,
                      }}
                    >
                      <div className="mb-2 flex items-center justify-between gap-3">
                        <span
                          style={{
                            color: toneForRunStatus(run.status).color,
                            background: toneForRunStatus(run.status).bg,
                            borderRadius: 999,
                            padding: '3px 9px',
                            fontSize: 10,
                            fontWeight: 800,
                            letterSpacing: '0.1em',
                            textTransform: 'uppercase',
                            fontFamily: "'Manrope', sans-serif",
                          }}
                        >
                          {run.status}
                        </span>
                        <span style={{ fontSize: 11, color: SHELL.muted, fontFamily: "'Space Grotesk', sans-serif" }}>{timeAgo(run.updated_at)}</span>
                      </div>
                      <div style={{ fontSize: 14, fontWeight: 900, color: SHELL.ink, fontFamily: "'Manrope', sans-serif" }}>
                        {shortAddress(run.token_address, 8, 6)}
                      </div>
                      <div style={{ marginTop: 6, fontSize: 12, lineHeight: 1.6, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                        {run.summary ?? 'Live run ready to reopen.'}
                      </div>
                    </Link>
                  ))
                ) : (
                  <div style={{ fontSize: 12, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                    No live priority lanes are visible yet.
                  </div>
                )}
              </div>
            </div>

            <div
              style={{
                borderRadius: 22,
                border: `1px solid ${SHELL.border}`,
                background: 'rgba(255,255,255,0.03)',
                padding: 18,
              }}
            >
              <div style={{ fontSize: 10, color: SHELL.muted, fontWeight: 800, letterSpacing: '0.16em', textTransform: 'uppercase', fontFamily: "'Manrope', sans-serif" }}>
                Deep entry
              </div>
              <div style={{ marginTop: 8, fontSize: 18, lineHeight: 1.2, fontWeight: 900, color: SHELL.ink, fontFamily: "'Manrope', sans-serif" }}>
                Need the full token console?
              </div>
              <div style={{ marginTop: 8, fontSize: 13, lineHeight: 1.7, color: SHELL.body, fontFamily: "'Space Grotesk', sans-serif" }}>
                Jump straight into the investigation workspace when a token deserves the full read.
              </div>
              <Link
                href={topToken ? `/mia?q=${encodeURIComponent(topToken.contract_address)}` : '/mia'}
                style={{
                  marginTop: 16,
                  width: '100%',
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 8,
                  padding: '12px 16px',
                  borderRadius: 14,
                  background: SHELL.blue,
                  color: '#081736',
                  fontSize: 11,
                  fontWeight: 800,
                  letterSpacing: '0.12em',
                  textTransform: 'uppercase',
                  fontFamily: "'Manrope', sans-serif",
                }}
              >
                Open MIA
                <FaArrowRight size={13} />
              </Link>
            </div>
          </aside>
        </div>
      </main>
    </>
  );
}
