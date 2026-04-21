'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import {
  FaChevronLeft,
  FaChevronRight,
  FaMagnifyingGlass,
  FaStar,
  FaWaveSquare,
} from 'react-icons/fa6';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { IntelligenceSummaryResponse, TokenSummary, WhaleAlertResponse } from '@/lib/types';

const POLL_INTERVAL_MS = 30_000;
const PAGE_SIZE = 6;

type RiskFilter = 'all' | 'low' | 'medium' | 'high';
type SortMode = 'newest' | 'volume' | 'risk';

function timeAgo(iso: string) {
  const diff = Date.now() - new Date(iso).getTime();
  const sec = Math.max(1, Math.floor(diff / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}

function safetyScore(token: TokenSummary) {
  if (token.composite_score === null) return 50;
  return Math.max(0, Math.min(100, 100 - token.composite_score));
}

function tone(token: TokenSummary) {
  if (token.risk_category === 'low') return { color: 'var(--secondary-container)' };
  if (token.risk_category === 'medium') return { color: 'var(--warning)' };
  return { color: 'var(--danger)' };
}

function tokenLabel(token: TokenSummary) {
  return token.symbol ?? token.name ?? token.contract_address;
}

function shortAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

export default function TokensPage() {
  const [rows, setRows] = useState<TokenSummary[]>([]);
  const [total, setTotal] = useState(0);
  const [narratives, setNarratives] = useState<Record<string, string | null>>({});
  const [whales, setWhales] = useState<WhaleAlertResponse[]>([]);
  const [summary, setSummary] = useState<IntelligenceSummaryResponse | null>(null);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [query, setQuery] = useState('');
  const [riskFilter, setRiskFilter] = useState<RiskFilter>('all');
  const [minLiquidity, setMinLiquidity] = useState(0);
  const [sortMode, setSortMode] = useState<SortMode>('newest');
  const [page, setPage] = useState(1);

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const currentPage = Math.min(page, totalPages);

  useEffect(() => {
    setPage(1);
  }, [query, riskFilter, minLiquidity, sortMode]);

  useEffect(() => {
    if (page > totalPages) setPage(totalPages);
  }, [page, totalPages]);

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const offset = (currentPage - 1) * PAGE_SIZE;
        const [tokenRes, whaleRes, summaryRes] = await Promise.all([
          api.tokens.list({
            limit: PAGE_SIZE,
            offset,
            ...(riskFilter !== 'all' ? { risk: riskFilter } : {}),
            ...(query.trim().length > 0 ? { q: query.trim() } : {}),
            ...(minLiquidity > 0 ? { min_liquidity: minLiquidity } : {}),
            sort: sortMode,
          }),
          api.whales.stream({ limit: 8, min_amount: 0.5 }),
          api.intelligence.summary(),
        ]);

        if (!active) return;

        setRows(tokenRes.data);
        setTotal(tokenRes.total);
        setWhales(whaleRes.data);
        setSummary(summaryRes);
        setError(null);

        if (tokenRes.data.length === 0) {
          setNarratives({});
        } else {
          const pairs = await Promise.all(
            tokenRes.data.map(async (token) => {
              try {
                const nar = await api.tokens.narrative(token.contract_address);
                return [token.contract_address, nar.narrative_text] as const;
              } catch {
                return [token.contract_address, null] as const;
              }
            })
          );
          if (active) {
            setNarratives(Object.fromEntries(pairs));
          }
        }
      } catch (err) {
        if (!active) return;
        setError(err instanceof Error ? err.message : 'Failed to load token feed');
      } finally {
        if (active) setLoading(false);
      }
    };

    load();
    const id = setInterval(load, POLL_INTERVAL_MS);

    return () => {
      active = false;
      clearInterval(id);
    };
  }, [currentPage, query, riskFilter, minLiquidity, sortMode]);

  const whaleCards = useMemo(() => whales.slice(0, 8), [whales]);
  const lowRiskShare =
    summary && summary.total_tokens > 0
      ? (summary.low_risk_tokens / summary.total_tokens) * 100
      : 0;

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page px-4 md:px-6 lg:px-8">
        <section className="mx-auto max-w-7xl">
          <section className="mb-8">
            <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
              Supporting Discovery Surface
            </p>
            <h1 className="font-headline mb-2 text-3xl font-extrabold tracking-tight md:text-4xl" data-testid="tokens-page-heading">Structured token index for deeper filtering</h1>
            <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
              Use this page when you want a more structured index than the main discover feed. The primary discovery entry stays in Discover, while this surface helps you filter and inspect the indexed universe more tightly.
            </p>
            <div className="mt-4 flex flex-wrap gap-2">
              <Link
                href="/app"
                data-testid="tokens-open-discover"
                className="rounded-lg px-3 py-2 text-xs font-semibold"
                style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
              >
                Open Discover
              </Link>
              <Link
                href="/mia"
                className="rounded-lg border px-3 py-2 text-xs font-semibold"
                style={{ background: 'var(--surface-container-low)', color: 'var(--primary)', borderColor: 'rgba(67,70,85,0.35)' }}
              >
                Open Investigation Workspace
              </Link>
            </div>
          </section>

          <section className="mb-6 grid grid-cols-1 gap-3 md:grid-cols-3">
            <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.35)' }}>
              <p className="text-[10px] font-headline font-bold uppercase tracking-widest" style={{ color: 'var(--outline)' }}>
                Coverage
              </p>
              <p className="mt-2 mono text-2xl font-bold" style={{ color: 'var(--primary)' }}>
                {(summary?.total_tokens ?? 0).toLocaleString()}
              </p>
              <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                Indexed tokens ready to be filtered into an investigation shortlist.
              </p>
            </div>
            <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.35)' }}>
              <p className="text-[10px] font-headline font-bold uppercase tracking-widest" style={{ color: 'var(--outline)' }}>
                Low-Risk Share
              </p>
              <p className="mt-2 mono text-2xl font-bold" style={{ color: 'var(--secondary-container)' }}>
                {lowRiskShare.toFixed(1)}%
              </p>
              <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                {summary?.low_risk_tokens ?? 0} low-risk tokens in the active indexed universe.
              </p>
            </div>
            <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.35)' }}>
              <p className="text-[10px] font-headline font-bold uppercase tracking-widest" style={{ color: 'var(--outline)' }}>
                Whale Alerts 24H
              </p>
              <p className="mt-2 mono text-2xl font-bold" style={{ color: 'var(--warning)' }}>
                {(summary?.total_whale_alerts_24h ?? 0).toLocaleString()}
              </p>
              <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                Large flow events that may justify opening or revisiting an investigation.
              </p>
            </div>
          </section>

          <section className="mb-5 grid grid-cols-1 gap-3 md:grid-cols-12">
            <div className="md:col-span-4">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Search token/address/deployer
              </label>
              <div className="relative">
                <FaMagnifyingGlass size={12} className="absolute left-3 top-1/2 -translate-y-1/2" style={{ color: 'var(--outline)' }} />
                <input
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="pepecat / 0xabc..."
                  className="w-full rounded-lg border py-2 pl-8 pr-3 text-sm"
                  style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.45)' }}
                />
              </div>
            </div>

            <div className="md:col-span-2">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Risk filter
              </label>
              <select
                value={riskFilter}
                onChange={(e) => setRiskFilter(e.target.value as RiskFilter)}
                className="w-full rounded-lg border px-3 py-2 text-sm"
                style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.45)' }}
              >
                <option value="all">All</option>
                <option value="low">Low</option>
                <option value="medium">Medium</option>
                <option value="high">High</option>
              </select>
            </div>

            <div className="md:col-span-2">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Sort
              </label>
              <select
                value={sortMode}
                onChange={(e) => setSortMode(e.target.value as SortMode)}
                className="w-full rounded-lg border px-3 py-2 text-sm"
                style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.45)' }}
              >
                <option value="newest">Newest</option>
                <option value="volume">Volume</option>
                <option value="risk">Risk</option>
              </select>
            </div>

            <div className="md:col-span-2">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Min liquidity
              </label>
              <input
                type="number"
                min={0}
                step={0.1}
                value={minLiquidity}
                onChange={(e) => setMinLiquidity(Number.isFinite(Number(e.target.value)) ? Number(e.target.value) : 0)}
                className="w-full rounded-lg border px-3 py-2 text-sm"
                style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.45)' }}
              />
            </div>

            <div className="md:col-span-2">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Page
              </label>
              <div className="mono rounded-lg border px-3 py-2 text-sm" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.45)' }}>
                {currentPage}/{totalPages}
              </div>
            </div>
          </section>

          <section className="mb-10 overflow-hidden">
            <div className="mb-4 flex items-center gap-2">
              <FaWaveSquare size={13} style={{ color: 'var(--primary)' }} />
              <h2 className="font-headline text-xs font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
                Whale Alerts
              </h2>
            </div>

            <div className="hide-scrollbar flex gap-4 overflow-x-auto pb-2">
              {whaleCards.length === 0 && (
                <div className="w-72 flex-shrink-0 rounded-xl border p-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.2)' }}>
                  <div className="mb-2 flex items-start justify-between">
                    <span className="mono text-xs font-bold" style={{ color: 'var(--secondary-container)' }}>WHALE_MONITOR</span>
                    <span className="mono text-[10px]" style={{ color: 'var(--on-surface-variant)' }}>live</span>
                  </div>
                  <div className="font-headline mb-1 text-sm font-bold">No event for current stream query.</div>
                  <Link href="/whales" className="mono text-xs hover:underline" style={{ color: 'var(--primary)' }}>
                    Open full whale stream
                  </Link>
                </div>
              )}

              {whaleCards.map((w) => (
                <div key={w.tx_hash} className="w-72 flex-shrink-0 rounded-xl border p-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.2)' }}>
                  <div className="mb-2 flex items-start justify-between">
                    <span className="mono text-xs font-bold" style={{ color: w.alert_level === 'critical' ? 'var(--warning)' : 'var(--secondary-container)' }}>
                      {w.alert_level === 'critical' ? 'LARGE_SWAP' : 'WHALE_ENTRY'}
                    </span>
                    <span className="mono text-[10px]" style={{ color: 'var(--on-surface-variant)' }}>{timeAgo(w.created_at)}</span>
                  </div>
                  <div className="font-headline mb-1 text-sm font-bold">{shortAddress(w.wallet_address, 8, 4)} bought {w.amount_bnb.toFixed(1)} BNB</div>
                  <Link href={`/mia?q=${encodeURIComponent(w.token_address)}`} className="mono text-xs hover:underline" style={{ color: 'var(--primary)' }}>
                    {shortAddress(w.token_address, 8, 4)}
                  </Link>
                </div>
              ))}
            </div>
          </section>

          <section className="space-y-6">
            <div className="mb-2 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="h-2 w-2 animate-pulse rounded-full" style={{ background: 'var(--secondary-container)' }} />
                <span className="font-headline text-xs font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>
                  Token Feed
                </span>
              </div>
              <span className="text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                {total} results
              </span>
            </div>

            {error && (
              <div className="rounded-xl p-4 text-sm" style={{ background: 'var(--surface-container-low)', color: 'var(--danger)' }}>
                {error}
              </div>
            )}

            {loading && rows.length === 0 && (
              <div className="rounded-xl p-4 text-sm" style={{ background: 'var(--surface-container-low)', color: 'var(--on-surface-variant)' }}>
                Loading intelligence feed...
              </div>
            )}

            {!loading && rows.length === 0 && !error && (
              <div className="rounded-xl p-4 text-sm" style={{ background: 'var(--surface-container-low)', color: 'var(--on-surface-variant)' }}>
                No token matches your filter.
              </div>
            )}

            {rows.map((token) => {
              const s = safetyScore(token);
              const ring = tone(token);
              const dashOffset = 176 * (1 - s / 100);
              const narrative = narratives[token.contract_address];

              return (
                <Link
                  key={token.contract_address}
                  href={`/mia?q=${encodeURIComponent(token.contract_address)}`}
                  className="group relative block rounded-xl p-6 transition-all duration-300 hover:opacity-95"
                  style={{ background: 'var(--surface-container-high)' }}
                >
                  <div className="grid grid-cols-1 items-center gap-6 md:grid-cols-12">
                    <div className="flex items-center gap-4 md:col-span-3">
                      <div className="relative flex h-16 w-16 items-center justify-center">
                        <svg className="h-full w-full -rotate-90">
                          <circle cx="32" cy="32" r="28" fill="transparent" stroke="var(--surface-variant)" strokeWidth="4" />
                          <circle cx="32" cy="32" r="28" fill="transparent" stroke={ring.color} strokeWidth="4" strokeDasharray="176" strokeDashoffset={dashOffset} />
                        </svg>
                        <span className="num absolute text-sm font-bold" style={{ color: ring.color }}>{Math.round(s)}</span>
                      </div>
                      <div>
                        <h3 className="mono text-lg font-bold">${tokenLabel(token)}</h3>
                        <p className="mono text-[10px]" style={{ color: 'var(--on-surface-variant)' }}>{timeAgo(token.deployed_at)}</p>
                      </div>
                    </div>

                    <div className="flex flex-col justify-center gap-1 border-l pl-6 md:col-span-3" style={{ borderColor: 'rgba(67,70,85,0.3)' }}>
                      <span className="text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Liquidity</span>
                      <span className="mono text-sm font-bold" style={{ color: 'var(--primary)' }}>{token.volume_bnb.toFixed(1)} BNB</span>
                      <span className="mono text-[10px]" style={{ color: 'var(--outline)' }}>{shortAddress(token.deployer_address, 10, 4)}</span>
                    </div>

                    <div className="rounded-lg border p-3 md:col-span-6" style={{ background: 'rgba(13,14,16,0.45)', borderColor: 'rgba(67,70,85,0.22)' }}>
                      <div className="mb-1 flex items-center gap-2">
                        <FaStar size={12} style={{ color: 'var(--primary)' }} />
                        <span className="text-[10px] font-bold uppercase tracking-tight" style={{ color: 'var(--primary)' }}>MIA Analysis</span>
                      </div>
                      <p className="text-xs italic leading-relaxed" style={{ color: 'var(--on-surface-variant)' }}>
                        {narrative ?? 'No narrative row returned by backend for this token.'}
                      </p>
                    </div>
                  </div>
                </Link>
              );
            })}

            <div className="mt-2 flex items-center justify-center gap-2">
              <button
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                disabled={currentPage <= 1}
                className="rounded-lg px-3 py-2 text-xs disabled:opacity-40"
                style={{ background: 'var(--surface-container-low)' }}
              >
                <FaChevronLeft className="mr-1 inline" size={10} /> Prev
              </button>

              <span className="num rounded-lg px-3 py-2 text-xs" style={{ background: 'var(--surface-container-low)' }}>
                {currentPage}/{totalPages}
              </span>

              <button
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={currentPage >= totalPages}
                className="rounded-lg px-3 py-2 text-xs disabled:opacity-40"
                style={{ background: 'var(--surface-container-low)' }}
              >
                Next <FaChevronRight className="ml-1 inline" size={10} />
              </button>
            </div>
          </section>
        </section>
      </main>
    </>
  );
}
