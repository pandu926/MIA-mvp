'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { AlphaRowResponse, TokenSummary, WhaleAlertResponse } from '@/lib/types';

function shortAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

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

export default function UserPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [summary, setSummary] = useState<{
    total_tokens: number;
    low_risk_tokens: number;
    medium_risk_tokens: number;
    high_risk_tokens: number;
    total_whale_alerts_24h: number;
    latest_alpha_window_end: string | null;
  } | null>(null);
  const [topTokens, setTopTokens] = useState<TokenSummary[]>([]);
  const [latestAlpha, setLatestAlpha] = useState<AlphaRowResponse[]>([]);
  const [latestWhales, setLatestWhales] = useState<WhaleAlertResponse[]>([]);

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const [sum, tokenRes, alphaRes, whaleRes] = await Promise.all([
          api.intelligence.summary(),
          api.tokens.list({ limit: 8, sort: 'volume', min_liquidity: 0.1 }),
          api.alpha.latest(5),
          api.whales.stream({ limit: 5, min_amount: 0.5 }),
        ]);

        if (!active) return;
        setSummary(sum);
        setTopTokens(tokenRes.data);
        setLatestAlpha(alphaRes);
        setLatestWhales(whaleRes.data);
        setError(null);
      } catch (e) {
        if (!active) return;
        setError(e instanceof Error ? e.message : 'Failed to load operator profile');
      } finally {
        if (active) setLoading(false);
      }
    };

    load();
    const id = setInterval(load, 20_000);

    return () => {
      active = false;
      clearInterval(id);
    };
  }, []);

  const riskMix = useMemo(() => {
    if (!summary) return { lowPct: 0, medPct: 0, highPct: 0 };
    const total = Math.max(1, summary.total_tokens);
    return {
      lowPct: (summary.low_risk_tokens / total) * 100,
      medPct: (summary.medium_risk_tokens / total) * 100,
      highPct: (summary.high_risk_tokens / total) * 100,
    };
  }, [summary]);

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page px-4 md:px-8">
        <section className="mx-auto max-w-7xl space-y-6">
          <article className="relative overflow-hidden rounded-xl p-8" style={{ background: 'var(--surface-container-low)' }}>
            <div className="absolute inset-0 opacity-20" style={{ background: 'linear-gradient(120deg, rgba(105,137,255,0.4), transparent)' }} />
            <div className="relative">
              <h1 className="font-headline text-4xl font-extrabold tracking-tight">Operator Profile</h1>
              <p className="mono mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                Runtime intelligence state of the MIA terminal.
              </p>

              {error && (
                <p className="mt-3 text-sm" style={{ color: 'var(--danger)' }}>
                  {error}
                </p>
              )}

              <div className="mt-4 grid grid-cols-2 gap-3 md:grid-cols-4">
                <Card label="Tracked Tokens" value={String(summary?.total_tokens ?? 0)} />
                <Card label="Whale Alerts 24H" value={String(summary?.total_whale_alerts_24h ?? 0)} />
                <Card label="Top Risk Bucket" value={
                  summary
                    ? summary.high_risk_tokens >= summary.medium_risk_tokens && summary.high_risk_tokens >= summary.low_risk_tokens
                      ? 'High'
                      : summary.medium_risk_tokens >= summary.low_risk_tokens
                        ? 'Medium'
                        : 'Low'
                    : 'N/A'
                } />
                <Card
                  label="Latest Alpha"
                  value={summary?.latest_alpha_window_end ? timeAgo(summary.latest_alpha_window_end) : 'N/A'}
                />
              </div>
            </div>
          </article>

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-12">
            <article className="glass-panel p-4 lg:col-span-7">
              <p className="mb-2 text-sm font-semibold">Top Tokens by Volume</p>
              {loading && topTokens.length === 0 && (
                <p className="text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                  Loading top tokens...
                </p>
              )}
              <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
                {!loading && topTokens.length === 0 && (
                  <p className="text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                    No tokens with liquidity above threshold yet.
                  </p>
                )}
                {topTokens.map((t) => (
                  <Link
                    key={t.contract_address}
                    href={`/mia?q=${encodeURIComponent(t.contract_address)}`}
                    className="rounded-lg p-3 hover:opacity-90"
                    style={{ background: 'var(--surface-container-high)' }}
                  >
                    <p className="font-headline text-sm font-bold">{t.symbol ?? t.name ?? shortAddress(t.contract_address)}</p>
                    <p className="mono mt-1 text-xs" style={{ color: 'var(--primary)' }}>
                      {t.volume_bnb.toFixed(2)} BNB
                    </p>
                    <p className="mono mt-1 text-[10px]" style={{ color: 'var(--on-surface-variant)' }}>
                      {shortAddress(t.contract_address, 10, 4)}
                    </p>
                  </Link>
                ))}
              </div>
            </article>

            <aside className="space-y-3 lg:col-span-5">
              <article className="obsidian-panel p-4">
                <p className="text-[10px] uppercase tracking-[0.18em]" style={{ color: 'var(--on-surface-variant)' }}>
                  Risk Distribution
                </p>
                <p className="mt-2 text-xs">Low: {riskMix.lowPct.toFixed(1)}%</p>
                <p className="mt-1 text-xs">Medium: {riskMix.medPct.toFixed(1)}%</p>
                <p className="mt-1 text-xs">High: {riskMix.highPct.toFixed(1)}%</p>
                <div className="mt-3 h-2 w-full overflow-hidden rounded" style={{ background: 'var(--surface-container-high)' }}>
                  <div className="h-full" style={{ width: `${riskMix.lowPct}%`, background: 'var(--secondary-container)', float: 'left' }} />
                  <div className="h-full" style={{ width: `${riskMix.medPct}%`, background: 'var(--warning)', float: 'left' }} />
                  <div className="h-full" style={{ width: `${riskMix.highPct}%`, background: 'var(--danger)', float: 'left' }} />
                </div>
              </article>

              <article className="obsidian-panel p-4">
                <p className="text-[10px] uppercase tracking-[0.18em]" style={{ color: 'var(--on-surface-variant)' }}>
                  Latest Alpha Picks
                </p>
                <div className="mt-2 space-y-2 text-xs">
                  {latestAlpha.map((a) => (
                    <Link key={`${a.window_end}-${a.rank}-${a.token_address}`} href={`/mia?q=${encodeURIComponent(a.token_address)}`} className="block rounded p-2 hover:opacity-90" style={{ background: 'var(--surface-container-high)' }}>
                      <p className="mono">#{a.rank} · {shortAddress(a.token_address, 8, 4)}</p>
                      <p className="num mt-1" style={{ color: 'var(--primary)' }}>Score {a.alpha_score.toFixed(2)}</p>
                    </Link>
                  ))}
                </div>
              </article>

              <article className="obsidian-panel p-4">
                <p className="text-[10px] uppercase tracking-[0.18em]" style={{ color: 'var(--on-surface-variant)' }}>
                  Recent Whale Events
                </p>
                <div className="mt-2 space-y-2 text-xs">
                  {latestWhales.map((w) => (
                    <Link key={w.tx_hash} href={`/whales/network?wallet=${w.wallet_address}`} className="block rounded p-2 hover:opacity-90" style={{ background: 'var(--surface-container-high)' }}>
                      <p className="mono">{shortAddress(w.wallet_address, 10, 4)}</p>
                      <p className="mt-1">{w.amount_bnb.toFixed(2)} BNB · {w.alert_level}</p>
                    </Link>
                  ))}
                </div>
              </article>
            </aside>
          </div>
        </section>
      </main>
    </>
  );
}

function Card({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg p-3" style={{ background: 'var(--surface-container-high)' }}>
      <p className="text-[10px] uppercase tracking-[0.15em]" style={{ color: 'var(--on-surface-variant)' }}>{label}</p>
      <p className="num mt-1 text-xl font-bold" style={{ color: 'var(--primary)' }}>{value}</p>
    </div>
  );
}
