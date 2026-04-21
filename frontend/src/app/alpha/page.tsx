'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import { FaChevronRight, FaClockRotateLeft } from 'react-icons/fa6';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { AlphaRowResponse } from '@/lib/types';

type Mode = 'live' | 'historical';

type AlphaDisplayRow = AlphaRowResponse & {
  symbolOrName: string;
};

function shortAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

export default function AlphaPage() {
  const [mode, setMode] = useState<Mode>('live');
  const [rows, setRows] = useState<AlphaDisplayRow[]>([]);
  const [backtest, setBacktest] = useState<{
    hit_rate_1h: number;
    hit_rate_6h: number;
    average_score_1h: number;
    evaluated: number;
    labeled_1h: number;
    reliable: boolean;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        setLoading(true);

        const [alphaRows, backtest] = await Promise.all([
          mode === 'live' ? api.alpha.latest(20) : api.alpha.history(48, 20),
          api.alpha.backtest(24, 120).catch(() => null),
        ]);

        const tokenMeta = await Promise.all(
          alphaRows.map(async (row) => {
            try {
              const token = await api.tokens.get(row.token_address);
              return [row.token_address, token.symbol ?? token.name ?? shortAddress(row.token_address)] as const;
            } catch {
              return [row.token_address, shortAddress(row.token_address)] as const;
            }
          })
        );

        const labelMap = Object.fromEntries(tokenMeta);
        const displayRows = alphaRows.map((row) => ({
          ...row,
          symbolOrName: labelMap[row.token_address] ?? shortAddress(row.token_address),
        }));

        if (!active) return;
        setRows(displayRows);
        if (backtest) {
          const labeled1h = backtest.rows.filter(
            (row) => row.future_volume_1h > 0 || row.future_buy_count_1h > 0 || row.future_sell_count_1h > 0
          ).length;
          const reliable = backtest.evaluated >= 30 && labeled1h / Math.max(1, backtest.evaluated) >= 0.25;
          setBacktest({
            hit_rate_1h: backtest.hit_rate_1h,
            hit_rate_6h: backtest.hit_rate_6h,
            average_score_1h: backtest.average_score_1h,
            evaluated: backtest.evaluated,
            labeled_1h: labeled1h,
            reliable,
          });
        } else {
          setBacktest(null);
        }
        setError(null);
      } catch (e) {
        if (!active) return;
        setError(e instanceof Error ? e.message : 'Failed to load alpha rankings');
      } finally {
        if (active) setLoading(false);
      }
    };

    load();

    if (mode === 'live') {
      const id = setInterval(load, 30_000);
      return () => {
        active = false;
        clearInterval(id);
      };
    }

    return () => {
      active = false;
    };
  }, [mode]);

  const featured = rows[0] ?? null;
  const listRows = useMemo(() => rows.slice(1), [rows]);

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page px-4 md:px-8">
        <section className="mx-auto max-w-7xl">
          <header className="mb-10 flex flex-col justify-between gap-6 md:flex-row md:items-end">
            <div className="space-y-2">
              <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
                Supporting Specialist Surface
              </p>
              <div className="mb-1 flex items-center gap-3">
                <span className="signal-chip safe">Consensus: Verified</span>
                <Link href="/backtesting" className="flex items-center gap-1 text-xs font-medium transition-colors" style={{ color: 'rgba(183,196,255,0.7)' }}>
                  <FaClockRotateLeft size={11} />
                  Backtesting Accuracy: {backtest !== null && backtest.reliable ? `${backtest.hit_rate_1h.toFixed(1)}%` : 'N/A'}
                </Link>
              </div>
              <h1 className="font-headline text-4xl font-extrabold tracking-tight md:text-5xl" data-testid="alpha-page-heading">Alpha ranking for fast prioritization</h1>
              <p className="max-w-2xl text-base" style={{ color: 'var(--on-surface-variant)' }}>
                Use this page to prioritize which launches deserve attention first. It supports Discover and Investigation, but it is not the primary way users should understand the product.
              </p>
              <div className="flex flex-wrap gap-2 pt-2">
                <Link
                  href="/app"
                  data-testid="alpha-open-discover"
                  className="rounded-lg px-3 py-2 text-xs font-semibold"
                  style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                >
                  Open Discover
                </Link>
                <Link
                  href="/backtesting"
                  className="rounded-lg border px-3 py-2 text-xs font-semibold"
                  style={{ background: 'var(--surface-container-low)', color: 'var(--primary)', borderColor: 'rgba(67,70,85,0.35)' }}
                >
                  Open Proof
                </Link>
              </div>
            </div>

            <div className="flex items-center gap-1 rounded-lg p-1" style={{ background: 'var(--surface-container-low)' }}>
              <button
                onClick={() => setMode('live')}
                className="rounded-md px-4 py-2 text-xs font-bold uppercase tracking-widest"
                style={{
                  color: mode === 'live' ? 'var(--primary)' : 'var(--on-surface-variant)',
                  background: mode === 'live' ? 'rgba(105,137,255,0.12)' : 'transparent',
                }}
              >
                Live
              </button>
              <button
                onClick={() => setMode('historical')}
                className="rounded-md px-4 py-2 text-xs font-bold uppercase tracking-widest"
                style={{
                  color: mode === 'historical' ? 'var(--primary)' : 'var(--on-surface-variant)',
                  background: mode === 'historical' ? 'rgba(105,137,255,0.12)' : 'transparent',
                }}
              >
                Historical
              </button>
            </div>
          </header>

          <section className="mb-6 grid grid-cols-1 gap-3 md:grid-cols-4">
            <MetricCard
              label="Hit Rate 1H"
              value={backtest && backtest.reliable ? `${backtest.hit_rate_1h.toFixed(1)}%` : 'N/A'}
              description={`${backtest?.evaluated ?? 0} evaluated, ${backtest?.labeled_1h ?? 0} labeled (1H)`}
              tone="var(--secondary-container)"
            />
            <MetricCard
              label="Hit Rate 6H"
              value={backtest && backtest.reliable ? `${backtest.hit_rate_6h.toFixed(1)}%` : 'N/A'}
              description="Validates whether the signal can hold up beyond the first hour."
              tone="var(--primary)"
            />
            <MetricCard
              label="Avg Score 1H"
              value={backtest && backtest.reliable ? backtest.average_score_1h.toFixed(2) : 'N/A'}
              description="Average realized alpha outcome across the 1H replay window."
              tone="var(--warning)"
            />
            <MetricCard
              label="Top Signal"
              value={featured ? featured.symbolOrName : 'N/A'}
              description={featured ? `Score ${featured.alpha_score.toFixed(1)} / 100` : 'Waiting for ranking window'}
              tone="var(--on-surface)"
            />
          </section>

          {error && (
            <div className="glass-panel mb-4 p-3 text-sm" style={{ color: 'var(--danger)' }}>
              {error}
            </div>
          )}

          {!error && loading && rows.length === 0 && (
            <div className="glass-panel mb-4 p-3 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
              Loading alpha feed...
            </div>
          )}

          {featured && (
            <article
              className="group relative mb-4 overflow-hidden rounded-xl border-l-4 p-6 shadow-2xl transition-all hover:translate-x-1"
              style={{ background: 'var(--surface-container-high)', borderLeftColor: 'var(--secondary-container)' }}
            >
              <div className="absolute -right-20 -top-20 h-64 w-64 rounded-full blur-3xl transition-colors group-hover:bg-white/5" style={{ background: 'rgba(105,137,255,0.08)' }} />

              <div className="relative flex flex-col justify-between gap-6 md:flex-row md:items-center">
                <div className="flex items-center gap-6">
                  <span className="mono text-4xl font-bold tracking-tighter" style={{ color: 'rgba(141,144,161,0.35)' }}>
                    {String(featured.rank).padStart(2, '0')}
                  </span>
                  <div>
                    <div className="flex items-center gap-2">
                      <h2 className="font-headline text-2xl font-bold tracking-tight">{featured.symbolOrName}</h2>
                      <span className="rounded-sm px-1.5 py-0.5 text-[10px] font-black" style={{ background: 'var(--secondary-container)', color: 'var(--on-secondary-container)' }}>
                        ALPHA ULTRA
                      </span>
                    </div>
                    <p className="mono mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>{shortAddress(featured.token_address, 12, 6)}</p>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-6 md:gap-12">
                  <div className="flex flex-col">
                    <span className="mb-1 text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Alpha Score</span>
                    <div className="flex items-baseline gap-1">
                      <span className="mono text-3xl font-bold" style={{ color: 'var(--secondary-container)' }}>{featured.alpha_score.toFixed(0)}</span>
                      <span className="mono text-sm" style={{ color: 'rgba(167,169,178,0.4)' }}>/100</span>
                    </div>
                  </div>

                  <div className="max-w-sm flex-1">
                    <span className="mb-1 text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>LLM Thesis</span>
                    <p className="text-sm italic leading-relaxed" style={{ color: 'var(--on-surface)' }}>
                      {featured.rationale}
                    </p>
                  </div>

                  <Link
                    href={`/mia?q=${encodeURIComponent(featured.token_address)}`}
                    data-testid="alpha-featured-open-investigation"
                    className="rounded-lg px-6 py-3 text-sm font-bold tracking-tight transition-all hover:brightness-110"
                    style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                  >
                    Open Investigation
                  </Link>
                </div>
              </div>
            </article>
          )}

          <div className="mt-4 space-y-3">
            {listRows.map((row) => (
              <Link
                key={`${row.window_end}-${row.rank}-${row.token_address}`}
                href={`/mia?q=${encodeURIComponent(row.token_address)}`}
                className="group flex items-center justify-between rounded-lg p-4 transition-all"
                style={{ background: 'var(--surface-container-low)' }}
              >
                <div className="flex flex-1 items-center gap-6">
                  <span className="mono w-6 text-center text-lg font-bold" style={{ color: 'rgba(141,144,161,0.45)' }}>
                    {String(row.rank).padStart(2, '0')}
                  </span>

                  <div className="flex min-w-[120px] flex-col">
                    <span className="font-headline text-lg font-bold">{row.symbolOrName}</span>
                    <span className="mono text-xs" style={{ color: 'var(--on-surface-variant)' }}>{shortAddress(row.token_address, 10, 4)}</span>
                  </div>

                  <div className="hidden max-w-md flex-1 md:block">
                    <p className="truncate text-sm" style={{ color: 'var(--on-surface-variant)' }}>{row.rationale}</p>
                  </div>
                </div>

                <div className="flex items-center gap-8">
                  <div className="flex flex-col items-end">
                    <span className="text-[9px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Score</span>
                    <span className="mono text-xl font-bold" style={{ color: 'var(--secondary-container)' }}>
                      {row.alpha_score.toFixed(0)}<span className="text-[10px]" style={{ color: 'rgba(167,169,178,0.4)' }}>/100</span>
                    </span>
                  </div>
                  <FaChevronRight size={14} className="transition-colors" style={{ color: 'var(--outline)' }} />
                </div>
              </Link>
            ))}
          </div>
        </section>
      </main>
    </>
  );
}

function MetricCard({
  label,
  value,
  description,
  tone,
}: {
  label: string;
  value: string;
  description: string;
  tone: string;
}) {
  return (
    <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.35)' }}>
      <p className="text-[10px] font-headline font-bold uppercase tracking-widest" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mono mt-2 text-2xl font-bold" style={{ color: tone }}>
        {value}
      </p>
      <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
        {description}
      </p>
    </div>
  );
}
