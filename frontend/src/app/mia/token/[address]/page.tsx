'use client';

import Link from 'next/link';
import { useParams } from 'next/navigation';
import { useEffect, useMemo, useState } from 'react';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { InvestigationRunSummary } from '@/lib/types';

function formatDate(value: string | null | undefined) {
  if (!value) return 'n/a';
  return new Date(value).toLocaleString();
}

function formatRelativeTime(value: string | null | undefined) {
  if (!value) return 'n/a';
  const diffMs = Date.now() - new Date(value).getTime();
  const diffSec = Math.max(1, Math.floor(diffMs / 1000));
  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${Math.floor(diffHours / 24)}d ago`;
}

function shortAddress(value: string, head = 10, tail = 6) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

function statusTone(status: string) {
  if (status === 'completed') {
    return { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' };
  }
  if (status === 'watching' || status === 'escalated') {
    return { color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' };
  }
  if (status === 'failed') {
    return { color: 'var(--danger)', background: 'rgba(255,107,107,0.12)' };
  }
  return { color: 'var(--warning)', background: 'rgba(255,186,73,0.12)' };
}

export default function MiaTokenHistoryPage() {
  const params = useParams<{ address: string }>();
  const address = decodeURIComponent(params.address);
  const [rows, setRows] = useState<InvestigationRunSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const response = await api.investigations.runs({ limit: 50, token: address });
        if (!active) return;
        setRows(response.data);
        setError(null);
      } catch (err) {
        if (!active) return;
        setError(err instanceof Error ? err.message : 'Failed to load token history');
      } finally {
        if (active) setLoading(false);
      }
    };

    void load();
    const id = window.setInterval(load, 30_000);

    return () => {
      active = false;
      window.clearInterval(id);
    };
  }, [address]);

  const summary = useMemo(() => {
    const latest = rows[0];
    return {
      count: rows.length,
      latestStatus: latest?.status ?? 'n/a',
      latestTrigger: latest?.trigger_type ?? 'n/a',
      latestUpdated: latest?.updated_at ?? null,
    };
  }, [rows]);

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
        <section>
          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
            Token History
          </p>
          <h1 className="mt-2 font-headline text-4xl font-extrabold tracking-tight" data-testid="mia-token-history-heading">
            Token investigation history
          </h1>
          <p className="mt-3 max-w-3xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
            This page shows how MIA has investigated the same token over time, so the workspace does not reset to zero every time you reopen it.
          </p>
        </section>

        <section
          className="rounded-2xl border p-5"
          style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
        >
          <div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
            <div className="space-y-3">
              <div>
                <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                  Token address
                </p>
                <p className="mt-2 text-sm font-semibold break-all" data-testid="mia-token-history-address">
                  {address}
                </p>
              </div>

              <div
                data-testid="mia-token-history-summary"
                className="grid gap-3 text-sm md:grid-cols-4"
              >
                <HistoryMetric label="Run count" value={String(summary.count)} />
                <HistoryMetric label="Latest status" value={summary.latestStatus} />
                <HistoryMetric label="Latest trigger" value={summary.latestTrigger} />
                <HistoryMetric
                  label="Updated"
                  value={summary.latestUpdated ? `${formatRelativeTime(summary.latestUpdated)} • ${formatDate(summary.latestUpdated)}` : 'n/a'}
                />
              </div>
            </div>

            <div className="flex flex-wrap gap-3">
              <Link
                href={`/mia?q=${encodeURIComponent(address)}`}
                className="rounded-xl px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
              >
                Open Investigation
              </Link>
              <Link
                href="/mia/runs"
                className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}
              >
                Back To Runs
              </Link>
            </div>
          </div>
        </section>

        {error && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}
          >
            {error}
          </section>
        )}

        {loading && rows.length === 0 && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
          >
            Loading token run history...
          </section>
        )}

        {!loading && rows.length === 0 && !error && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
          >
            No stored investigation history exists for this token yet.
          </section>
        )}

        <section className="grid gap-4">
          {rows.map((run) => (
            <article
              key={run.run_id}
              data-testid="mia-token-history-row"
              className="rounded-2xl border p-5"
              style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
            >
              <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                <div className="space-y-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={statusTone(run.status)}>
                      {run.status}
                    </span>
                    <span
                      className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                      style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }}
                    >
                      {run.trigger_type}
                    </span>
                    <span
                      className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                      style={{ color: 'var(--on-surface-variant)', background: 'rgba(255,255,255,0.04)' }}
                    >
                      {run.current_stage}
                    </span>
                  </div>

                  <div>
                    <h2 className="font-headline text-xl font-bold tracking-tight">
                      {run.current_read ?? 'Investigation run'} • {shortAddress(run.run_id)}
                    </h2>
                    <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                      {run.summary ?? 'No summary is attached to this run yet.'}
                    </p>
                  </div>

                  <div className="grid gap-3 text-sm md:grid-cols-4">
                    <HistoryMetric label="Run ID" value={run.run_id} />
                    <HistoryMetric label="Token" value={run.token_address} />
                    <HistoryMetric label="Score" value={run.investigation_score !== null ? `${run.investigation_score}/100` : 'n/a'} />
                    <HistoryMetric label="Updated" value={`${formatRelativeTime(run.updated_at)} • ${formatDate(run.updated_at)}`} />
                  </div>
                </div>

                <div className="flex flex-wrap gap-2">
                  <Link
                    href={`/mia/runs/${encodeURIComponent(run.run_id)}`}
                    className="rounded-lg border px-3 py-2 text-xs font-semibold"
                    style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}
                  >
                    Open Run Detail
                  </Link>
                  <Link
                    href={`/mia?q=${encodeURIComponent(run.token_address)}`}
                    className="rounded-lg px-3 py-2 text-xs font-semibold"
                    style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                  >
                    Open Investigation
                  </Link>
                </div>
              </div>
            </article>
          ))}
        </section>
      </main>
    </>
  );
}

function HistoryMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}>
      <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mt-2 break-all text-sm font-semibold leading-6">{value}</p>
    </div>
  );
}
