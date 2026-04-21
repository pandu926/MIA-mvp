'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import { FaChevronLeft, FaChevronRight, FaWaveSquare } from 'react-icons/fa6';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { WhaleAlertResponse, WhaleStreamResponse } from '@/lib/types';

const PAGE_SIZE = 12;

type LevelFilter = 'all' | 'watch' | 'critical';

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

function shortAddress(value: string, head = 8, tail = 6) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

export default function WhalesPage() {
  const [stream, setStream] = useState<WhaleStreamResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [minAmount, setMinAmount] = useState(0.5);
  const [levelFilter, setLevelFilter] = useState<LevelFilter>('all');
  const [tokenFilter, setTokenFilter] = useState('');
  const [page, setPage] = useState(1);

  useEffect(() => {
    setPage(1);
  }, [minAmount, levelFilter, tokenFilter]);

  useEffect(() => {
    let active = true;
    const offset = (page - 1) * PAGE_SIZE;

    const load = async () => {
      try {
        const res = await api.whales.stream({
          limit: PAGE_SIZE,
          offset,
          min_amount: minAmount,
          ...(levelFilter !== 'all' ? { level: levelFilter } : {}),
          ...(tokenFilter.trim().length > 0 ? { token: tokenFilter.trim() } : {}),
        });

        if (!active) return;
        setStream(res);
        setError(null);
      } catch (e) {
        if (!active) return;
        setError(e instanceof Error ? e.message : 'Failed to load whale stream');
      } finally {
        if (active) setLoading(false);
      }
    };

    load();
    const id = setInterval(load, 15_000);

    return () => {
      active = false;
      clearInterval(id);
    };
  }, [page, minAmount, levelFilter, tokenFilter]);

  const rows = useMemo<WhaleAlertResponse[]>(() => stream?.data ?? [], [stream]);
  const featured = rows[0] ?? null;
  const rest = rows.slice(1);

  const totalPages = Math.max(1, Math.ceil((stream?.total ?? 0) / PAGE_SIZE));
  const currentPage = Math.min(page, totalPages);

  useEffect(() => {
    if (page > totalPages) setPage(totalPages);
  }, [page, totalPages]);

  const displayedVolume = useMemo(
    () => rows.reduce((sum, item) => sum + item.amount_bnb, 0),
    [rows]
  );

  const activeWhales = useMemo(
    () => new Set(rows.map((item) => item.wallet_address.toLowerCase())).size,
    [rows]
  );

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page px-4 md:px-6 lg:px-8">
        <section className="mx-auto max-w-7xl">
          <header className="mb-6 flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
            <div>
              <div className="mb-1 flex items-center gap-2">
                <span className="live-dot" />
                <span className="mono text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--secondary-container)' }}>
                  Live Whale Stream
                </span>
              </div>
              <h1 className="font-headline text-3xl font-extrabold tracking-tight">Whale Intelligence Feed</h1>
            </div>
            <p className="text-xs" style={{ color: 'var(--on-surface-variant)' }}>
              {stream?.total ?? 0} events matched
            </p>
          </header>

          <section className="mb-6 grid grid-cols-1 gap-3 md:grid-cols-12">
            <div className="md:col-span-4">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Token address filter
              </label>
              <input
                value={tokenFilter}
                onChange={(e) => setTokenFilter(e.target.value)}
                placeholder="0x..."
                className="w-full rounded-lg border px-3 py-2 text-sm"
                style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.45)' }}
              />
            </div>

            <div className="md:col-span-3">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Alert level
              </label>
              <select
                value={levelFilter}
                onChange={(e) => setLevelFilter(e.target.value as LevelFilter)}
                className="w-full rounded-lg border px-3 py-2 text-sm"
                style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.45)' }}
              >
                <option value="all">All</option>
                <option value="watch">Watch</option>
                <option value="critical">Critical</option>
              </select>
            </div>

            <div className="md:col-span-3">
              <label className="mb-1 block text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
                Min amount (BNB)
              </label>
              <input
                type="number"
                min={0}
                step={0.1}
                value={minAmount}
                onChange={(e) => setMinAmount(Number.isFinite(Number(e.target.value)) ? Number(e.target.value) : 0)}
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

          <div className="grid grid-cols-1 gap-4 md:grid-cols-12">
            <article className="glass-panel relative overflow-hidden rounded-xl border p-6 md:col-span-8" style={{ borderColor: 'rgba(67,70,85,0.2)' }}>
              {featured ? (
                <div className="flex h-full flex-col justify-between gap-6">
                  <div>
                    <div className="mb-5 flex items-center gap-4">
                      <div className="flex h-12 w-12 items-center justify-center rounded-lg" style={{ background: 'rgba(105,137,255,0.2)' }}>
                        <FaWaveSquare size={20} style={{ color: 'var(--primary)' }} />
                      </div>
                      <div>
                        <Link
                          href={`/whales/network?wallet=${featured.wallet_address}`}
                          className="mono text-xs hover:underline"
                          style={{ color: 'var(--outline)' }}
                        >
                          {shortAddress(featured.wallet_address, 12, 6)}
                        </Link>
                        <h2 className="font-headline text-xl font-bold" style={{ color: 'var(--primary)' }}>
                          Largest Event On Page
                        </h2>
                      </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
                      <Metric label="Amount" value={`${featured.amount_bnb.toFixed(2)} BNB`} tone="var(--secondary-container)" />
                      <Metric label="Level" value={featured.alert_level.toUpperCase()} />
                      <Metric label="Token" value={shortAddress(featured.token_address, 8, 4)} />
                      <Metric label="Time" value={timeAgo(featured.created_at)} />
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center justify-between gap-2 border-t pt-4" style={{ borderColor: 'rgba(67,70,85,0.2)' }}>
                    <Link
                      href={`/mia?q=${encodeURIComponent(featured.token_address)}`}
                      className="rounded-lg px-4 py-2 text-xs font-bold"
                      style={{ background: 'rgba(105,137,255,0.2)', color: 'var(--primary)' }}
                    >
                      OPEN TOKEN
                    </Link>
                    <Link
                      href="/whales/network"
                      className="rounded-lg px-4 py-2 text-xs font-bold"
                      style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                    >
                      VIEW NETWORK
                    </Link>
                  </div>
                </div>
              ) : (
                <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                  {loading ? 'Loading whale signal...' : 'No whale signal matches your filter.'}
                </p>
              )}
            </article>

            <aside className="flex flex-col gap-4 md:col-span-4">
              <div className="rounded-xl border p-5" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.2)' }}>
                <p className="mb-1 text-[10px] font-bold uppercase" style={{ color: 'var(--outline)' }}>Displayed Volume</p>
                <p className="mono text-2xl font-bold" style={{ color: 'var(--primary)' }}>
                  {displayedVolume.toFixed(2)} <span className="text-base">BNB</span>
                </p>
              </div>

              <div className="rounded-xl border p-5" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.2)' }}>
                <p className="mb-1 text-[10px] font-bold uppercase" style={{ color: 'var(--outline)' }}>Active Wallets (page)</p>
                <p className="mono text-2xl font-bold">{activeWhales}</p>
              </div>
            </aside>

            <section className="md:col-span-12">
              <div className="overflow-hidden rounded-xl border" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.2)' }}>
                <div className="flex items-center justify-between border-b px-6 py-4" style={{ borderColor: 'rgba(67,70,85,0.2)', background: 'rgba(41,42,44,0.4)' }}>
                  <h3 className="text-sm font-bold uppercase tracking-wide">Stream Events</h3>
                  <div className="flex items-center gap-2">
                    <span className="h-2 w-2 rounded-full" style={{ background: 'var(--secondary-container)' }} />
                    <span className="text-[10px] font-bold uppercase" style={{ color: 'var(--outline)' }}>Live</span>
                  </div>
                </div>

                {error && (
                  <p className="px-6 py-3 text-sm" style={{ color: 'var(--danger)' }}>
                    {error}
                  </p>
                )}

                {!loading && rows.length === 0 && !error && (
                  <p className="px-6 py-4 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                    No whale event found for this filter.
                  </p>
                )}

                <div>
                  {rest.map((item) => (
                    <div
                      key={item.tx_hash}
                      className="flex flex-wrap items-center justify-between gap-4 px-6 py-4 md:flex-nowrap"
                      style={{ borderTop: '1px solid rgba(67,70,85,0.14)' }}
                    >
                      <div className="flex min-w-[220px] items-center gap-4">
                        <span
                          className="mono rounded px-2 py-1 text-xs font-bold"
                          style={{
                            color: item.alert_level === 'critical' ? 'var(--danger)' : 'var(--secondary-container)',
                            background: item.alert_level === 'critical' ? 'rgba(255,107,107,0.12)' : 'rgba(0,255,163,0.12)',
                          }}
                        >
                          {item.alert_level.toUpperCase()}
                        </span>
                        <div>
                          <Link href={`/whales/network?wallet=${item.wallet_address}`} className="mono text-sm font-bold hover:underline">
                            {shortAddress(item.wallet_address, 12, 4)}
                          </Link>
                          <p className="text-[10px] font-bold uppercase" style={{ color: 'var(--outline)' }}>
                            Whale wallet
                          </p>
                        </div>
                      </div>

                      <div className="flex flex-col md:items-end">
                        <p
                          className="mono text-sm font-bold"
                          style={{ color: item.alert_level === 'critical' ? 'var(--danger)' : 'var(--secondary-container)' }}
                        >
                          {item.amount_bnb.toFixed(2)} BNB
                        </p>
                      </div>

                      <div className="flex flex-col md:items-end">
                        <Link href={`/mia?q=${encodeURIComponent(item.token_address)}`} className="mono text-sm font-bold hover:underline">
                          {shortAddress(item.token_address, 8, 4)}
                        </Link>
                        <p className="text-[10px] font-bold uppercase" style={{ color: 'var(--outline)' }}>
                          {timeAgo(item.created_at)}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              <div className="mt-4 flex items-center justify-center gap-2">
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
          </div>
        </section>
      </main>
    </>
  );
}

function Metric({ label, value, tone }: { label: string; value: string; tone?: string }) {
  return (
    <div>
      <p className="text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mono mt-1 text-sm font-bold" style={{ color: tone ?? 'var(--on-background)' }}>
        {value}
      </p>
    </div>
  );
}
