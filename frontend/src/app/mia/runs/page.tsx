'use client';

import Link from 'next/link';
import { useEffect, useState } from 'react';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { InvestigationOpsSummaryResponse, InvestigationRunSummary } from '@/lib/types';

type RunStatusFilter = 'all' | 'queued' | 'running' | 'completed' | 'failed' | 'watching' | 'escalated' | 'archived';
type RunTriggerFilter = 'all' | 'manual' | 'auto';

const STATUS_FILTERS: Array<{ value: RunStatusFilter; label: string }> = [
  { value: 'all', label: 'All runs' },
  { value: 'queued', label: 'Queued' },
  { value: 'completed', label: 'Completed' },
  { value: 'watching', label: 'Watching' },
  { value: 'escalated', label: 'Escalated' },
  { value: 'archived', label: 'Archived' },
  { value: 'running', label: 'Running' },
  { value: 'failed', label: 'Failed' },
];

const TRIGGER_FILTERS: Array<{ value: RunTriggerFilter; label: string }> = [
  { value: 'all', label: 'All triggers' },
  { value: 'manual', label: 'Manual' },
  { value: 'auto', label: 'Auto' },
];

function shortAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

function timeAgo(value: string) {
  const diff = Date.now() - new Date(value).getTime();
  const sec = Math.max(1, Math.floor(diff / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}

function formatMinutes(value: number | null) {
  if (value === null || Number.isNaN(value)) return 'No completed runs yet';
  return `${value.toFixed(1)} min avg`;
}

function formatPercent(value: number) {
  return `${value.toFixed(1)}%`;
}

function statusTone(status: string) {
  if (status === 'completed') {
    return { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' };
  }
  if (status === 'watching' || status === 'escalated') {
    return { color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' };
  }
  if (status === 'archived') {
    return { color: 'var(--outline)', background: 'rgba(255,255,255,0.06)' };
  }
  if (status === 'failed') {
    return { color: 'var(--danger)', background: 'rgba(255,107,107,0.12)' };
  }
  return { color: 'var(--warning)', background: 'rgba(255,186,73,0.12)' };
}

export default function MiaRunsPage() {
  const [rows, setRows] = useState<InvestigationRunSummary[]>([]);
  const [opsSummary, setOpsSummary] = useState<InvestigationOpsSummaryResponse | null>(null);
  const [statusFilter, setStatusFilter] = useState<RunStatusFilter>('all');
  const [triggerFilter, setTriggerFilter] = useState<RunTriggerFilter>('all');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [controlNotice, setControlNotice] = useState<string | null>(null);
  const [controlLoading, setControlLoading] = useState(false);
  const [cleanupLoading, setCleanupLoading] = useState(false);
  const [retryLoading, setRetryLoading] = useState(false);
  const [recoveryLoading, setRecoveryLoading] = useState(false);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    setStatusFilter(normalizeStatusFilter(params.get('status')));
    setTriggerFilter(normalizeTriggerFilter(params.get('trigger')));
  }, []);

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const [response, summary] = await Promise.all([
          api.investigations.runs({
            limit: 50,
            status: statusFilter === 'all' ? undefined : statusFilter,
            trigger: triggerFilter === 'all' ? undefined : triggerFilter,
          }),
          api.investigations.opsSummary(),
        ]);
        if (!active) return;
        setRows(response.data);
        setOpsSummary(summary);
        setError(null);
      } catch (err) {
        if (!active) return;
        setError(err instanceof Error ? err.message : 'Failed to load runs inbox');
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
  }, [statusFilter, triggerFilter]);

  const toggleAutoInvestigation = async () => {
    if (!opsSummary) return;
    setControlLoading(true);
    setControlNotice(null);
    try {
      const updated = await api.investigations.updateOpsControl({
        auto_investigation_paused: !opsSummary.auto_investigation.paused,
      });
      setOpsSummary(updated);
      setControlNotice(
        updated.auto_investigation.paused
          ? 'Auto investigation is paused for operator review.'
          : 'Auto investigation resumed.'
      );
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update operator control');
    } finally {
      setControlLoading(false);
    }
  };

  const archiveStaleRuns = async () => {
    setCleanupLoading(true);
    setControlNotice(null);
    try {
      const response = await api.investigations.archiveStaleRuns({ stale_after_minutes: 0 });
      setOpsSummary(response.ops_summary);
      setControlNotice(
        response.archived_count > 0
          ? `Archived ${response.archived_count} terminal run${response.archived_count === 1 ? '' : 's'} from the inbox.`
          : 'No terminal runs were ready for archive cleanup.'
      );

      const refreshed = await api.investigations.runs({
        limit: 50,
        status: statusFilter === 'all' ? undefined : statusFilter,
        trigger: triggerFilter === 'all' ? undefined : triggerFilter,
      });
      setRows(refreshed.data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to archive stale runs');
    } finally {
      setCleanupLoading(false);
    }
  };

  const retryFailedRuns = async () => {
    setRetryLoading(true);
    setControlNotice(null);
    try {
      const response = await api.investigations.retryFailedRuns();
      setOpsSummary(response.ops_summary);
      setControlNotice(
        response.retried_count > 0
          ? `Re-queued ${response.retried_count} failed run${response.retried_count === 1 ? '' : 's'} for another investigation pass.`
          : 'No failed runs were waiting for retry.'
      );

      const refreshed = await api.investigations.runs({
        limit: 50,
        status: statusFilter === 'all' ? undefined : statusFilter,
        trigger: triggerFilter === 'all' ? undefined : triggerFilter,
      });
      setRows(refreshed.data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to retry failed runs');
    } finally {
      setRetryLoading(false);
    }
  };

  const recoverStaleRunningRuns = async () => {
    setRecoveryLoading(true);
    setControlNotice(null);
    try {
      const response = await api.investigations.recoverStaleRunningRuns({ stale_after_minutes: 30 });
      setOpsSummary(response.ops_summary);
      setControlNotice(
        response.recovered_count > 0
          ? `Recovered ${response.recovered_count} stale running run${response.recovered_count === 1 ? '' : 's'} back into queue.`
          : 'No stale running runs needed recovery.'
      );

      const refreshed = await api.investigations.runs({
        limit: 50,
        status: statusFilter === 'all' ? undefined : statusFilter,
        trigger: triggerFilter === 'all' ? undefined : triggerFilter,
      });
      setRows(refreshed.data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to recover stale running runs');
    } finally {
      setRecoveryLoading(false);
    }
  };

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page min-h-screen px-3 pb-16 pt-4 md:px-8">
        <div className="mx-auto max-w-[440px] space-y-6 lg:hidden">
          <section>
            <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
              Runs
            </p>
            <h1 className="mt-2 font-headline text-4xl font-extrabold tracking-tight" data-testid="mia-runs-heading">
              Operate live investigations from one console
            </h1>
            <p className="mt-3 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
              This is the operating surface for MIA runs. Reopen recent investigations, inspect loop health, use operator controls, and keep continuity instead of starting from zero every time.
            </p>
          </section>

          <section className="rounded-2xl border p-5" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}>
            <div className="space-y-5">
              <div>
                <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                  Status filter
                </p>
                <div className="mt-3 flex flex-wrap gap-2">
                  {STATUS_FILTERS.map((filter) => (
                    <FilterChip key={filter.value} testId={`mia-runs-status-${filter.value}`} label={filter.label} active={statusFilter === filter.value} onClick={() => setStatusFilter(filter.value)} />
                  ))}
                </div>
              </div>

              <div>
                <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                  Trigger filter
                </p>
                <div className="mt-3 flex flex-wrap gap-2">
                  {TRIGGER_FILTERS.map((filter) => (
                    <FilterChip key={filter.value} testId={`mia-runs-trigger-${filter.value}`} label={filter.label} active={triggerFilter === filter.value} onClick={() => setTriggerFilter(filter.value)} />
                  ))}
                </div>
              </div>

              <div data-testid="mia-runs-filter-summary" className="rounded-xl border px-4 py-3 text-sm" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}>
                Showing <span className="font-semibold">{rows.length}</span> run{rows.length === 1 ? '' : 's'} for <span className="font-semibold">{statusFilter}</span> / <span className="font-semibold">{triggerFilter}</span>.
              </div>
            </div>
          </section>

          {opsSummary && (
            <section data-testid="mia-runs-ops-summary" className="rounded-2xl border p-5" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}>
              <div className="mb-4">
                <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                  Operator summary
                </p>
                <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                  A calm snapshot of what is live right now: run states, trigger mix, saved watch context, saved missions, and the active auto-investigation policy.
                </p>
              </div>

              <div className="mb-4 space-y-3">
                <div data-testid="mia-runs-auto-state" className="rounded-xl border px-4 py-3 text-sm" style={{ background: opsSummary.auto_investigation.paused ? 'rgba(255,186,73,0.12)' : 'rgba(0,255,163,0.08)', borderColor: opsSummary.auto_investigation.paused ? 'rgba(255,186,73,0.24)' : 'rgba(0,255,163,0.24)', color: opsSummary.auto_investigation.paused ? 'var(--warning)' : 'var(--secondary-container)' }}>
                  {opsSummary.auto_investigation.paused ? 'Auto investigation paused' : 'Auto investigation live'}
                </div>

                <div className="grid gap-3">
                  <button type="button" data-testid="mia-runs-toggle-auto-investigation" onClick={() => void toggleAutoInvestigation()} disabled={controlLoading} className="rounded-xl px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ background: opsSummary.auto_investigation.paused ? 'var(--primary-container)' : 'rgba(255,186,73,0.14)', color: opsSummary.auto_investigation.paused ? 'var(--on-primary-container)' : 'var(--warning)' }}>
                    {controlLoading ? 'Updating...' : opsSummary.auto_investigation.paused ? 'Resume auto investigation' : 'Pause auto investigation'}
                  </button>
                  <button type="button" data-testid="mia-runs-retry-failed" onClick={() => void retryFailedRuns()} disabled={retryLoading} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(105,137,255,0.18)', background: 'rgba(105,137,255,0.08)', color: 'var(--primary)' }}>
                    {retryLoading ? 'Retrying...' : 'Retry failed runs'}
                  </button>
                  <button type="button" data-testid="mia-runs-archive-stale" onClick={() => void archiveStaleRuns()} disabled={cleanupLoading} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(141,144,161,0.18)', background: 'var(--surface-container-lowest)', color: 'var(--on-surface)' }}>
                    {cleanupLoading ? 'Archiving...' : 'Archive terminal runs'}
                  </button>
                  <button type="button" data-testid="mia-runs-recover-stale" onClick={() => void recoverStaleRunningRuns()} disabled={recoveryLoading} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(0,255,163,0.18)', background: 'rgba(0,255,163,0.08)', color: 'var(--secondary-container)' }}>
                    {recoveryLoading ? 'Recovering...' : 'Recover stale running'}
                  </button>
                </div>
              </div>

              {controlNotice && (
                <div data-testid="mia-runs-control-notice" className="mb-4 rounded-xl border px-4 py-3 text-sm" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)', color: 'var(--primary)' }}>
                  {controlNotice}
                </div>
              )}

              {opsSummary.degradation_notes.length > 0 && (
                <div className="mb-4 space-y-3">
                  {opsSummary.degradation_notes.map((note) => (
                    <div key={note.code} data-testid="mia-runs-degradation-note" className="rounded-xl border px-4 py-3 text-sm" style={{ background: note.level === 'warn' ? 'rgba(255,186,73,0.12)' : 'rgba(105,137,255,0.08)', borderColor: note.level === 'warn' ? 'rgba(255,186,73,0.2)' : 'rgba(105,137,255,0.16)', color: note.level === 'warn' ? 'var(--warning)' : 'var(--primary)' }}>
                      {note.message}
                    </div>
                  ))}
                </div>
              )}

              <div className="grid gap-3">
                <OpsCard title="Run states" testId="mia-runs-ops-run-states" lines={[`Queued ${opsSummary.runs.queued}`, `Running ${opsSummary.runs.running}`, `Watching ${opsSummary.runs.watching}`, `Escalated ${opsSummary.runs.escalated}`, `Completed ${opsSummary.runs.completed}`, `Failed ${opsSummary.runs.failed}`]} />
                <OpsCard title="Trigger mix" testId="mia-runs-ops-trigger-mix" lines={[`Manual ${opsSummary.triggers.manual}`, `Auto ${opsSummary.triggers.auto}`, `Archived ${opsSummary.runs.archived}`]} />
                <OpsCard title="Loop health" testId="mia-runs-ops-loop-health" lines={[`Auto starts 24h ${opsSummary.loop_health.auto_runs_24h}`, `Retries 24h ${opsSummary.loop_health.retry_actions_24h}`, `Recoveries 24h ${opsSummary.loop_health.recovery_actions_24h}`, `Failure rate 24h ${formatPercent(opsSummary.loop_health.failure_rate_24h_pct)}`, `Avg completion ${formatMinutes(opsSummary.loop_health.average_completion_minutes_24h)}`]} />
                <OpsCard title="Operator context" testId="mia-runs-ops-operator-context" lines={[`Watchlist ${opsSummary.watchlist_items}`, `Mission active ${opsSummary.missions.active}`, `Mission paused ${opsSummary.missions.paused}`, `Mission archived ${opsSummary.missions.archived}`]} />
                <OpsCard title="Auto policy" testId="mia-runs-ops-auto-policy" lines={[opsSummary.auto_investigation.enabled ? opsSummary.auto_investigation.paused ? 'Auto investigation enabled but paused' : 'Auto investigation enabled' : 'Auto investigation disabled', `Threshold ${opsSummary.auto_investigation.tx_threshold} tx`, `Cooldown ${opsSummary.auto_investigation.cooldown_mins} min`]} />
              </div>
            </section>
          )}

          {error && <section className="rounded-xl border p-4 text-sm" style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}>{error}</section>}
          {loading && rows.length === 0 && <section className="rounded-xl border p-4 text-sm" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}>Loading runs inbox...</section>}
          {!loading && rows.length === 0 && !error && <section className="rounded-xl border p-4 text-sm" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}>No investigation runs exist yet. Open a token investigation from Discover or `/mia` to create the first run.</section>}

          <section className="grid gap-4">
            {rows.map((run) => {
              const tone = statusTone(run.status);
              return (
                <article key={run.run_id} data-testid="mia-run-row" className="rounded-2xl border p-5" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}>
                  <div className="space-y-4">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={tone}>{run.status}</span>
                      <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }}>{run.trigger_type}</span>
                      <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--on-surface-variant)', background: 'rgba(255,255,255,0.04)' }}>{run.source_surface}</span>
                    </div>
                    <div>
                      <h2 className="font-headline text-xl font-bold tracking-tight">{run.current_read ?? 'Investigation run'} • {shortAddress(run.token_address, 10, 6)}</h2>
                      <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>{run.summary ?? 'No summary is attached to this run yet.'}</p>
                    </div>
                    <div className="grid gap-3 text-sm">
                      <MiniRow label="Run ID" value={run.run_id} />
                      <MiniRow label="Token" value={run.token_address} />
                      <MiniRow label="Current stage" value={run.current_stage} />
                      <MiniRow label="Investigation score" value={run.investigation_score !== null ? `${run.investigation_score}/100` : 'n/a'} />
                      <MiniRow label="Updated" value={`${timeAgo(run.updated_at)} • ${new Date(run.updated_at).toLocaleString()}`} />
                    </div>
                    <div className="grid gap-2">
                      <Link href={`/mia/runs/${encodeURIComponent(run.run_id)}`} className="rounded-lg border px-3 py-3 text-center text-xs font-semibold" style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}>Open Run Detail</Link>
                      <Link href={`/mia/token/${encodeURIComponent(run.token_address)}`} className="rounded-lg border px-3 py-3 text-center text-xs font-semibold" style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}>View Token History</Link>
                      <Link href={`/mia?q=${encodeURIComponent(run.token_address)}`} className="rounded-lg px-3 py-3 text-center text-xs font-semibold" style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}>Open Investigation</Link>
                    </div>
                  </div>
                </article>
              );
            })}
          </section>
        </div>

        <div className="mx-auto hidden max-w-[1420px] gap-6 lg:grid lg:grid-cols-[260px_minmax(0,1fr)_340px]">
          <aside className="sticky top-24 h-fit space-y-4">
            <section className="rounded-[24px] border p-6" style={{ background: 'rgba(18,24,38,0.92)', borderColor: 'rgba(255,255,255,0.06)' }}>
              <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
                Runs
              </p>
              <h1 className="mt-3 font-headline text-[2rem] font-extrabold leading-[1.02] tracking-tight" data-testid="mia-runs-heading">
                Run console for live investigations
              </h1>
              <p className="mt-3 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                Watch the system, reopen continuity, and operate runs from one calm surface instead of a stretched ops table.
              </p>
            </section>

            <section className="rounded-[24px] border p-5" style={{ background: 'rgba(23,31,49,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}>
              <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                Status filter
              </p>
              <div className="mt-3 grid gap-2">
                {STATUS_FILTERS.map((filter) => (
                  <FilterChip key={filter.value} testId={`mia-runs-status-${filter.value}`} label={filter.label} active={statusFilter === filter.value} onClick={() => setStatusFilter(filter.value)} />
                ))}
              </div>
            </section>

            <section className="rounded-[24px] border p-5" style={{ background: 'rgba(23,31,49,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}>
              <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                Trigger filter
              </p>
              <div className="mt-3 grid gap-2">
                {TRIGGER_FILTERS.map((filter) => (
                  <FilterChip key={filter.value} testId={`mia-runs-trigger-${filter.value}`} label={filter.label} active={triggerFilter === filter.value} onClick={() => setTriggerFilter(filter.value)} />
                ))}
              </div>
            </section>
          </aside>

          <section className="min-w-0 space-y-5">
            <section className="rounded-[28px] border p-6" style={{ background: 'linear-gradient(135deg, rgba(111,141,255,0.15), rgba(18,24,38,0.96))', borderColor: 'rgba(111,141,255,0.2)' }}>
              <div className="flex items-start justify-between gap-5">
                <div>
                  <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                    Operator focus
                  </p>
                  <div className="mt-3 text-[2rem] font-black leading-[1.02]" style={{ color: 'var(--on-surface)' }}>
                    Keep continuity across manual and auto runs.
                  </div>
                  <p className="mt-3 max-w-2xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                    Filter the live queue, inspect what changed, and jump into the right investigation without losing context.
                  </p>
                </div>
                <div data-testid="mia-runs-filter-summary" className="rounded-2xl border px-5 py-4 text-sm" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}>
                  Showing <span className="font-semibold">{rows.length}</span> run{rows.length === 1 ? '' : 's'} for <span className="font-semibold">{statusFilter}</span> / <span className="font-semibold">{triggerFilter}</span>.
                </div>
              </div>
            </section>

            {error && <section className="rounded-xl border p-4 text-sm" style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}>{error}</section>}
            {loading && rows.length === 0 && <section className="rounded-xl border p-4 text-sm" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}>Loading runs inbox...</section>}
            {!loading && rows.length === 0 && !error && <section className="rounded-xl border p-4 text-sm" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}>No investigation runs exist yet. Open a token investigation from Discover or `/mia` to create the first run.</section>}

            <section className="space-y-4">
              {rows.map((run) => {
                const tone = statusTone(run.status);
                return (
                  <article key={run.run_id} data-testid="mia-run-row" className="rounded-[24px] border p-5" style={{ background: 'rgba(23,31,49,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}>
                    <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_250px]">
                      <div className="min-w-0">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={tone}>{run.status}</span>
                          <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }}>{run.trigger_type}</span>
                          <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--on-surface-variant)', background: 'rgba(255,255,255,0.04)' }}>{run.source_surface}</span>
                        </div>

                        <div className="mt-4">
                          <h2 className="font-headline text-2xl font-bold tracking-tight">
                            {run.current_read ?? 'Investigation run'} • {shortAddress(run.token_address, 10, 6)}
                          </h2>
                          <p className="mt-2 max-w-2xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                            {run.summary ?? 'No summary is attached to this run yet.'}
                          </p>
                        </div>

                        <div className="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-5">
                          <MiniRow label="Run ID" value={run.run_id} />
                          <MiniRow label="Token" value={run.token_address} />
                          <MiniRow label="Current stage" value={run.current_stage} />
                          <MiniRow label="Investigation score" value={run.investigation_score !== null ? `${run.investigation_score}/100` : 'n/a'} />
                          <MiniRow label="Updated" value={`${timeAgo(run.updated_at)} • ${new Date(run.updated_at).toLocaleString()}`} />
                        </div>
                      </div>

                      <div className="grid gap-3 self-start">
                        <Link href={`/mia?q=${encodeURIComponent(run.token_address)}`} className="rounded-[16px] px-4 py-3 text-center text-xs font-extrabold uppercase tracking-[0.14em]" style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}>
                          Open Investigation
                        </Link>
                        <Link href={`/mia/runs/${encodeURIComponent(run.run_id)}`} className="rounded-[16px] border px-4 py-3 text-center text-xs font-semibold" style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}>
                          Open Run Detail
                        </Link>
                        <Link href={`/mia/token/${encodeURIComponent(run.token_address)}`} className="rounded-[16px] border px-4 py-3 text-center text-xs font-semibold" style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}>
                          View Token History
                        </Link>
                      </div>
                    </div>
                  </article>
                );
              })}
            </section>
          </section>

          <aside className="sticky top-24 h-fit space-y-4">
            {opsSummary && (
              <>
                <section data-testid="mia-runs-ops-summary" className="rounded-[24px] border p-5" style={{ background: 'rgba(23,31,49,0.92)', borderColor: 'rgba(255,255,255,0.06)' }}>
                  <div className="mb-4">
                    <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                      Auto state
                    </p>
                    <div data-testid="mia-runs-auto-state" className="mt-3 rounded-xl border px-4 py-3 text-sm" style={{ background: opsSummary.auto_investigation.paused ? 'rgba(255,186,73,0.12)' : 'rgba(0,255,163,0.08)', borderColor: opsSummary.auto_investigation.paused ? 'rgba(255,186,73,0.24)' : 'rgba(0,255,163,0.24)', color: opsSummary.auto_investigation.paused ? 'var(--warning)' : 'var(--secondary-container)' }}>
                      {opsSummary.auto_investigation.paused ? 'Auto investigation paused' : 'Auto investigation live'}
                    </div>
                  </div>

                  <div className="grid gap-3">
                    <button type="button" data-testid="mia-runs-toggle-auto-investigation" onClick={() => void toggleAutoInvestigation()} disabled={controlLoading} className="rounded-[16px] px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ background: opsSummary.auto_investigation.paused ? 'var(--primary-container)' : 'rgba(255,186,73,0.14)', color: opsSummary.auto_investigation.paused ? 'var(--on-primary-container)' : 'var(--warning)' }}>
                      {controlLoading ? 'Updating...' : opsSummary.auto_investigation.paused ? 'Resume auto investigation' : 'Pause auto investigation'}
                    </button>
                    <button type="button" data-testid="mia-runs-retry-failed" onClick={() => void retryFailedRuns()} disabled={retryLoading} className="rounded-[16px] border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(105,137,255,0.18)', background: 'rgba(105,137,255,0.08)', color: 'var(--primary)' }}>
                      {retryLoading ? 'Retrying...' : 'Retry failed runs'}
                    </button>
                    <button type="button" data-testid="mia-runs-archive-stale" onClick={() => void archiveStaleRuns()} disabled={cleanupLoading} className="rounded-[16px] border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(141,144,161,0.18)', background: 'var(--surface-container-lowest)', color: 'var(--on-surface)' }}>
                      {cleanupLoading ? 'Archiving...' : 'Archive terminal runs'}
                    </button>
                    <button type="button" data-testid="mia-runs-recover-stale" onClick={() => void recoverStaleRunningRuns()} disabled={recoveryLoading} className="rounded-[16px] border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(0,255,163,0.18)', background: 'rgba(0,255,163,0.08)', color: 'var(--secondary-container)' }}>
                      {recoveryLoading ? 'Recovering...' : 'Recover stale running'}
                    </button>
                  </div>
                </section>

                {controlNotice && <div data-testid="mia-runs-control-notice" className="rounded-xl border px-4 py-3 text-sm" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)', color: 'var(--primary)' }}>{controlNotice}</div>}

                {opsSummary.degradation_notes.length > 0 && (
                  <div className="space-y-3">
                    {opsSummary.degradation_notes.map((note) => (
                      <div key={note.code} data-testid="mia-runs-degradation-note" className="rounded-xl border px-4 py-3 text-sm" style={{ background: note.level === 'warn' ? 'rgba(255,186,73,0.12)' : 'rgba(105,137,255,0.08)', borderColor: note.level === 'warn' ? 'rgba(255,186,73,0.2)' : 'rgba(105,137,255,0.16)', color: note.level === 'warn' ? 'var(--warning)' : 'var(--primary)' }}>
                        {note.message}
                      </div>
                    ))}
                  </div>
                )}

                <section className="grid gap-4">
                  <OpsCard title="Run states" testId="mia-runs-ops-run-states" lines={[`Queued ${opsSummary.runs.queued}`, `Running ${opsSummary.runs.running}`, `Watching ${opsSummary.runs.watching}`, `Escalated ${opsSummary.runs.escalated}`, `Completed ${opsSummary.runs.completed}`, `Failed ${opsSummary.runs.failed}`]} />
                  <OpsCard title="Trigger mix" testId="mia-runs-ops-trigger-mix" lines={[`Manual ${opsSummary.triggers.manual}`, `Auto ${opsSummary.triggers.auto}`, `Archived ${opsSummary.runs.archived}`]} />
                  <OpsCard title="Loop health" testId="mia-runs-ops-loop-health" lines={[`Auto starts 24h ${opsSummary.loop_health.auto_runs_24h}`, `Retries 24h ${opsSummary.loop_health.retry_actions_24h}`, `Recoveries 24h ${opsSummary.loop_health.recovery_actions_24h}`, `Failure rate 24h ${formatPercent(opsSummary.loop_health.failure_rate_24h_pct)}`, `Avg completion ${formatMinutes(opsSummary.loop_health.average_completion_minutes_24h)}`]} />
                  <OpsCard title="Operator context" testId="mia-runs-ops-operator-context" lines={[`Watchlist ${opsSummary.watchlist_items}`, `Mission active ${opsSummary.missions.active}`, `Mission paused ${opsSummary.missions.paused}`, `Mission archived ${opsSummary.missions.archived}`]} />
                  <OpsCard title="Auto policy" testId="mia-runs-ops-auto-policy" lines={[opsSummary.auto_investigation.enabled ? opsSummary.auto_investigation.paused ? 'Auto investigation enabled but paused' : 'Auto investigation enabled' : 'Auto investigation disabled', `Threshold ${opsSummary.auto_investigation.tx_threshold} tx`, `Cooldown ${opsSummary.auto_investigation.cooldown_mins} min`]} />
                </section>
              </>
            )}
          </aside>
        </div>
      </main>
    </>
  );
}

function normalizeStatusFilter(value: string | null): RunStatusFilter {
  const allowed = new Set<RunStatusFilter>(['all', 'queued', 'running', 'completed', 'failed', 'watching', 'escalated', 'archived']);
  return value && allowed.has(value as RunStatusFilter) ? (value as RunStatusFilter) : 'all';
}

function normalizeTriggerFilter(value: string | null): RunTriggerFilter {
  const allowed = new Set<RunTriggerFilter>(['all', 'manual', 'auto']);
  return value && allowed.has(value as RunTriggerFilter) ? (value as RunTriggerFilter) : 'all';
}

function FilterChip({
  testId,
  label,
  active,
  onClick,
}: {
  testId: string;
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      data-testid={testId}
      aria-pressed={active}
      onClick={onClick}
      className="rounded-full border px-3 py-2 text-[11px] font-bold uppercase tracking-[0.16em] transition-colors"
      style={
        active
          ? {
              color: 'var(--primary)',
              background: 'rgba(105,137,255,0.12)',
              borderColor: 'rgba(105,137,255,0.22)',
            }
          : {
              color: 'var(--on-surface-variant)',
              background: 'rgba(255,255,255,0.03)',
              borderColor: 'rgba(141,144,161,0.12)',
            }
      }
    >
      {label}
    </button>
  );
}

function MiniRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mt-1 break-all text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
        {value}
      </p>
    </div>
  );
}

function OpsCard({ title, lines, testId }: { title: string; lines: string[]; testId?: string }) {
  return (
    <article
      data-testid={testId ?? 'mia-runs-ops-card'}
      className="rounded-xl border p-4"
      style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
    >
      <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
        {title}
      </p>
      <div className="mt-3 space-y-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
        {lines.map((line) => (
          <p key={line}>{line}</p>
        ))}
      </div>
    </article>
  );
}
