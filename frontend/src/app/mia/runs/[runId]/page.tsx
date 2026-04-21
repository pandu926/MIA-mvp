'use client';

import Link from 'next/link';
import { useParams } from 'next/navigation';
import { useEffect, useState } from 'react';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { InvestigationRunDetailResponse } from '@/lib/types';

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

function signalTagLabel(value: string | null | undefined) {
  switch (value) {
    case 'multi_signal':
      return 'Multi-signal';
    case 'builder_overlap':
      return 'Builder overlap';
    case 'source_degradation':
      return 'Source degradation';
    case 'linked_launch_overlap':
      return 'Linked launch overlap';
    case 'whale_alert':
      return 'Whale alert';
    case 'wallet_concentration':
      return 'Wallet concentration';
    case 'activity':
      return 'Activity';
    default:
      return 'Run signal';
  }
}

export default function MiaRunDetailPage() {
  const params = useParams<{ runId: string }>();
  const runId = params.runId;
  const [detail, setDetail] = useState<InvestigationRunDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const activeSignalTag = detail?.run.signal_tag ?? null;

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const response = await api.investigations.getRunDetail(runId);
        if (!active) return;
        setDetail(response);
        setError(null);
      } catch (err) {
        if (!active) return;
        setError(err instanceof Error ? err.message : 'Failed to load run detail');
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
  }, [runId]);

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
        <section>
          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
            Run Detail
          </p>
          <h1 className="mt-2 font-headline text-4xl font-extrabold tracking-tight" data-testid="mia-run-detail-heading">
            Investigation run detail
          </h1>
          <p className="mt-3 max-w-3xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
            This page turns one run into a clearer object: current state, continuity note, and a lightweight timeline of how the run progressed.
          </p>
        </section>

        {error && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}
          >
            {error}
          </section>
        )}

        {loading && !detail && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
          >
            Loading run detail...
          </section>
        )}

        {detail && (
          <>
            <section
              data-testid="mia-run-detail-summary"
              className="rounded-2xl border p-5"
              style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
            >
              <div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
                <div className="space-y-3">
                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      data-testid="mia-run-detail-status"
                      className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                      style={statusTone(detail.run.status)}
                    >
                      {detail.run.status}
                    </span>
                    <span
                      className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                      style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }}
                    >
                      {detail.run.trigger_type}
                    </span>
                    <span
                      className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                      style={{ color: 'var(--on-surface-variant)', background: 'rgba(255,255,255,0.04)' }}
                    >
                      {detail.run.current_stage}
                    </span>
                    {activeSignalTag && <SignalBadge value={activeSignalTag} testId="mia-run-detail-signal-badge" />}
                  </div>

                  <div>
                    <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                      Run identity
                    </p>
                    <p className="mt-2 text-sm font-semibold break-all" data-testid="mia-run-detail-id">
                      {detail.run.run_id}
                    </p>
                  </div>

                  <div className="grid gap-3 text-sm md:grid-cols-4">
                    <Metric label="Token" value={detail.run.token_address} />
                    <Metric label="Current read" value={detail.run.current_read ?? 'n/a'} />
                    <Metric label="Score" value={detail.run.investigation_score !== null ? `${detail.run.investigation_score}/100` : 'n/a'} />
                    <Metric label="Updated" value={`${formatRelativeTime(detail.run.updated_at)} • ${formatDate(detail.run.updated_at)}`} />
                  </div>

                  <div
                    className="rounded-xl border p-4 text-sm"
                    style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}
                  >
                    {detail.continuity_note}
                  </div>
                </div>

                <div className="flex flex-wrap gap-3">
                  <Link
                    href={`/mia?q=${encodeURIComponent(detail.run.token_address)}`}
                    className="rounded-xl px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                    style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                  >
                    Open Investigation
                  </Link>
                  <Link
                    href={`/mia/token/${encodeURIComponent(detail.run.token_address)}`}
                    className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                    style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}
                  >
                    Open Token History
                  </Link>
                </div>
              </div>
            </section>

            <section
              className="rounded-2xl border p-5"
              style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
            >
              <div className="flex items-center gap-2">
                <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                  Timeline
                </p>
              </div>
              <div className="mt-4 space-y-3">
                {detail.timeline.map((event) => {
                  return (
                  <article
                    key={event.key}
                    data-testid="mia-run-timeline-event"
                    className="rounded-xl border p-4"
                    style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
                  >
                    <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                      <div>
                        <div className="flex flex-wrap items-center gap-2">
                          <p className="font-semibold">{event.label}</p>
                          {event.signal_tag && <SignalBadge value={event.signal_tag} testId="mia-run-detail-timeline-signal-badge" />}
                        </div>
                        <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                          {event.detail}
                        </p>
                        {(event.reason || event.evidence_delta) && (
                          <div className="mt-3 grid gap-3 md:grid-cols-2">
                            <Metric label="Reason" value={event.reason ?? 'n/a'} />
                            <Metric label="Evidence delta" value={event.evidence_delta ?? 'n/a'} />
                          </div>
                        )}
                      </div>
                      <div className="text-xs" style={{ color: 'var(--outline)' }}>
                        {formatRelativeTime(event.at)} • {formatDate(event.at)}
                      </div>
                    </div>
                  </article>
                  );
                })}
              </div>
            </section>
          </>
        )}
      </main>
    </>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}>
      <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mt-2 break-all text-sm font-semibold leading-6">{value}</p>
    </div>
  );
}

function SignalBadge({ value, testId }: { value: string; testId?: string }) {
  return (
    <span
      data-testid={testId}
      className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
      style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }}
    >
      {signalTagLabel(value)}
    </span>
  );
}
