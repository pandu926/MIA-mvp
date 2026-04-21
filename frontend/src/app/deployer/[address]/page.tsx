'use client';

import Link from 'next/link';
import { use, useEffect, useMemo, useState } from 'react';
import {
  FaArrowLeft,
  FaChartLine,
  FaChevronRight,
  FaCopy,
  FaFilter,
  FaMedal,
  FaShieldHalved,
  FaStar,
} from 'react-icons/fa6';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { DeployerResponse, DeployerTokenResponse } from '@/lib/types';

interface Props {
  params: Promise<{ address: string }>;
}

function shortAddress(value: string, head = 10, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString();
}

function statusForToken(token: DeployerTokenResponse) {
  if (token.risk_category === 'low') return { label: 'Graduated', tone: 'safe' as const };
  if (token.buy_count >= token.sell_count && token.volume_bnb > 0) return { label: 'Active', tone: 'warn' as const };
  return { label: 'Failed', tone: 'danger' as const };
}

function roiProxy(token: DeployerTokenResponse) {
  const flowFactor = (token.buy_count + 1) / (token.sell_count + 1);
  const volumeFactor = 1 + token.volume_bnb / 10;
  return Math.max(0.1, flowFactor * volumeFactor);
}

export default function DeployerPage({ params }: Props) {
  const route = use(params);
  const routeAddress = route.address;

  const [profile, setProfile] = useState<DeployerResponse | null>(null);
  const [tokens, setTokens] = useState<DeployerTokenResponse[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let active = true;
    (async () => {
      try {
        const [deployer, deployedTokens] = await Promise.all([
          api.deployer.get(routeAddress),
          api.deployer.tokens(routeAddress, 50),
        ]);
        if (!active) return;
        setProfile(deployer);
        setTokens(deployedTokens);
        setError(null);
      } catch (e) {
        if (active) setError(e instanceof Error ? e.message : 'Failed to load deployer profile');
      }
    })();
    return () => {
      active = false;
    };
  }, [routeAddress]);

  const stats = useMemo(() => {
    const total = profile?.total_tokens_deployed ?? tokens.length;
    const graduated = profile?.graduated_count ?? tokens.filter((t) => t.risk_category === 'low').length;
    const rug = profile?.rug_count ?? 0;
    const gradRate = total > 0 ? (graduated / total) * 100 : 0;

    const roiValues = tokens.map(roiProxy);
    const avgRoi = roiValues.length > 0 ? roiValues.reduce((a, b) => a + b, 0) / roiValues.length : 0;

    return {
      total,
      graduated,
      rug,
      gradRate,
      avgRoi,
    };
  }, [profile, tokens]);

  const gradeView = useMemo(() => {
    const g = profile?.trust_grade ?? 'C';
    const tone = g === 'A' || g === 'B' ? 'var(--secondary-container)' : g === 'C' ? 'var(--primary)' : 'var(--danger)';
    const label = g === 'A' ? 'ELITE' : g === 'B' ? 'STRONG' : g === 'C' ? 'MODERATE' : 'HIGH RISK';
    return { grade: g, tone, label };
  }, [profile]);

  const copyAddress = async () => {
    try {
      await navigator.clipboard.writeText(routeAddress);
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      setCopied(false);
    }
  };

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page px-4 md:px-8">
        <section className="mx-auto max-w-5xl space-y-6">
          <section className="mb-8 flex flex-col justify-between gap-6 md:flex-row md:items-end">
            <div className="flex flex-col gap-2">
              <div className="flex items-center gap-3">
                <div className="flex h-12 w-12 items-center justify-center rounded-xl" style={{ background: 'var(--surface-container-high)', color: 'var(--primary)' }}>
                  <FaShieldHalved size={20} />
                </div>
                <div>
                  <h2 className="font-headline text-2xl font-extrabold tracking-tight">Deployer Profile</h2>
                  <p className="mono flex items-center gap-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                    {shortAddress(routeAddress, 12, 6)}
                    <button onClick={copyAddress} className="rounded p-1 hover:bg-white/5" title="Copy address">
                      <FaCopy size={11} />
                    </button>
                    {copied && <span className="text-[10px]" style={{ color: 'var(--secondary-container)' }}>copied</span>}
                  </p>
                </div>
              </div>
            </div>

            <div className="group relative">
              <div className="absolute inset-0 rounded-full opacity-60 blur-xl transition-opacity group-hover:opacity-100" style={{ background: 'rgba(0,255,163,0.2)' }} />
              <div className="relative flex items-center gap-4 rounded-xl border px-8 py-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(0,255,163,0.12)' }}>
                <div className="flex flex-col">
                  <span className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--on-surface-variant)' }}>Intelligence Grade</span>
                  <span className="font-headline text-4xl font-extrabold" style={{ color: gradeView.tone }}>
                    GRADE {gradeView.grade}
                  </span>
                </div>
                <div className="h-12 w-px" style={{ background: 'rgba(67,70,85,0.25)' }} />
                <div className="flex flex-col items-center">
                  <FaMedal size={22} style={{ color: gradeView.tone }} />
                  <span className="text-[10px] font-bold" style={{ color: gradeView.tone }}>{gradeView.label}</span>
                </div>
              </div>
            </div>
          </section>

          {error && <article className="glass-panel p-4 text-sm" style={{ color: 'var(--danger)' }}>{error}</article>}

          <section className="mb-10 grid grid-cols-1 gap-4 md:grid-cols-4">
            <MetricCard label="Total Launches" value={String(stats.total)} note={stats.total > 0 ? `${Math.round((stats.graduated / Math.max(1, stats.total)) * 100)}% survived` : 'N/A'} noteTone="var(--secondary-container)" />
            <MetricCard label="Graduation Rate" value={`${stats.gradRate.toFixed(0)}%`} note={stats.gradRate >= 60 ? 'Top tier' : 'Developing'} noteTone="var(--secondary-container)" />
            <MetricCard label="Avg ROI Proxy" value={`${stats.avgRoi.toFixed(1)}x`} note="Flow-weighted" noteTone="var(--primary)" />
            <MetricCard label="Rug Count" value={String(stats.rug)} note={stats.rug === 0 ? 'Pristine' : 'Watchlist'} noteTone={stats.rug === 0 ? 'var(--secondary-container)' : 'var(--danger)'} />
          </section>

          <section className="overflow-hidden rounded-xl border" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(67,70,85,0.14)' }}>
            <div className="flex items-center justify-between border-b px-6 py-5" style={{ borderColor: 'rgba(67,70,85,0.14)' }}>
              <h3 className="font-headline text-lg font-bold">Deployment History</h3>
              <div className="flex gap-2">
                <button className="rounded px-3 py-1.5 text-xs font-semibold" style={{ background: 'var(--surface-container-high)', color: 'var(--on-surface-variant)' }}>
                  <FaFilter className="mr-1 inline" size={10} /> Filters
                </button>
                <button className="rounded px-3 py-1.5 text-xs font-semibold" style={{ background: 'var(--surface-container-high)', color: 'var(--on-surface-variant)' }}>
                  <FaArrowLeft className="mr-1 inline -rotate-90" size={10} /> Export
                </button>
              </div>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-left">
                <thead style={{ background: 'var(--surface-container-low)' }}>
                  <tr>
                    <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Token Name</th>
                    <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Launch Date</th>
                    <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Peak ROI</th>
                    <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Status</th>
                    <th className="px-6 py-4 text-right text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {tokens.map((t) => {
                    const st = statusForToken(t);
                    const roi = roiProxy(t);
                    return (
                      <tr key={t.contract_address} style={{ borderTop: '1px solid rgba(67,70,85,0.14)' }} className="hover:bg-white/5">
                        <td className="px-6 py-4">
                          <div className="flex items-center gap-3">
                            <div className="flex h-8 w-8 items-center justify-center rounded-lg" style={{ background: 'var(--surface-container-highest)' }}>
                              <FaStar size={11} style={{ color: 'var(--primary)' }} />
                            </div>
                            <div>
                              <div className="font-headline text-sm font-bold">{t.symbol ?? t.name ?? 'UNKNOWN'}</div>
                              <div className="mono text-[10px]" style={{ color: 'var(--on-surface-variant)' }}>{shortAddress(t.contract_address, 8, 4)}</div>
                            </div>
                          </div>
                        </td>
                        <td className="px-6 py-4 mono text-sm" style={{ color: 'var(--on-surface-variant)' }}>{formatDate(t.deployed_at)}</td>
                        <td className="px-6 py-4 mono text-sm" style={{ color: roi >= 2 ? 'var(--secondary-container)' : roi >= 1 ? 'var(--primary)' : 'var(--danger)' }}>
                          {roi.toFixed(1)}x
                        </td>
                        <td className="px-6 py-4">
                          <span className={`signal-chip ${st.tone}`}>{st.label}</span>
                        </td>
                        <td className="px-6 py-4 text-right">
                          <Link href={`/mia?q=${encodeURIComponent(t.contract_address)}`} className="inline-flex rounded p-1 transition-colors hover:bg-white/10" style={{ color: 'var(--primary)' }}>
                            <FaChartLine size={16} />
                          </Link>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>

            <div className="p-4 text-center" style={{ background: 'rgba(27,28,30,0.6)' }}>
              <button className="text-xs font-bold uppercase tracking-widest hover:underline" style={{ color: 'var(--primary)' }}>
                View More Deployments <FaChevronRight className="ml-1 inline" size={10} />
              </button>
            </div>
          </section>
        </section>
      </main>
    </>
  );
}

function MetricCard({ label, value, note, noteTone }: { label: string; value: string; note: string; noteTone: string }) {
  return (
    <div className="rounded-xl border p-5" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.14)' }}>
      <span className="mb-4 block text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>{label}</span>
      <div className="flex items-baseline gap-2">
        <span className="font-headline text-3xl font-bold">{value}</span>
        <span className="mono text-xs" style={{ color: noteTone }}>{note}</span>
      </div>
    </div>
  );
}
