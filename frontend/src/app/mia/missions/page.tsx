'use client';

import Link from 'next/link';
import { useEffect, useState } from 'react';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { CreateInvestigationMissionRequest, InvestigationMissionResponse } from '@/lib/types';

const MISSION_TEMPLATES: Array<{
  type: CreateInvestigationMissionRequest['mission_type'];
  label: string;
  description: string;
}> = [
  {
    type: 'watch_hot_launches',
    label: 'Watch hot launches',
    description: 'Keep this context active for fresh launch movement, activity spikes, and run continuity.',
  },
  {
    type: 'watch_builder_cluster',
    label: 'Watch builder cluster',
    description: 'Track linked launches, deployer recurrence, and continuity around one builder context.',
  },
  {
    type: 'watch_suspicious_recurrence',
    label: 'Watch suspicious recurrence',
    description: 'Preserve a mission around recurring patterns that deserve repeated review over time.',
  },
  {
    type: 'watch_proof_qualified_launches',
    label: 'Watch proof-qualified launches',
    description: 'Keep a long-lived mission for launches that still deserve proof-layer monitoring.',
  },
];

function missionTypeLabel(value: string) {
  return MISSION_TEMPLATES.find((template) => template.type === value)?.label ?? value;
}

function statusTone(status: string) {
  if (status === 'active') {
    return { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' };
  }
  if (status === 'paused') {
    return { color: 'var(--warning)', background: 'rgba(255,186,73,0.12)' };
  }
  return { color: 'var(--outline)', background: 'rgba(255,255,255,0.05)' };
}

function relativeTime(value: string) {
  const diffMs = Date.now() - new Date(value).getTime();
  const diffSec = Math.max(1, Math.floor(diffMs / 1000));
  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${Math.floor(diffHours / 24)}d ago`;
}

export default function MiaMissionsPage() {
  const [missions, setMissions] = useState<InvestigationMissionResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [createLoadingType, setCreateLoadingType] = useState<string | null>(null);
  const [statusLoadingId, setStatusLoadingId] = useState<string | null>(null);
  const [missionContext, setMissionContext] = useState<{
    entityKind: 'token' | 'builder' | null;
    entityKey: string | null;
    label: string | null;
    sourceWatchlistItemId: string | null;
    sourceRunId: string | null;
  }>({
    entityKind: null,
    entityKey: null,
    label: null,
    sourceWatchlistItemId: null,
    sourceRunId: null,
  });

  const load = async () => {
    try {
      const response = await api.investigations.missions();
      setMissions(response.data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load missions');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    setMissionContext({
      entityKind: (params.get('entity_kind') as 'token' | 'builder' | null) ?? null,
      entityKey: params.get('entity_key'),
      label: params.get('label'),
      sourceWatchlistItemId: params.get('source_watchlist_item_id'),
      sourceRunId: params.get('source_run_id'),
    });
  }, []);

  const createMission = async (missionType: CreateInvestigationMissionRequest['mission_type']) => {
    setCreateLoadingType(missionType);
    setNotice(null);
    try {
      const template = MISSION_TEMPLATES.find((item) => item.type === missionType);
      const label = missionContext.label ? `${template?.label ?? missionType}: ${missionContext.label}` : template?.label;
      await api.investigations.createMission({
        mission_type: missionType,
        entity_kind: missionContext.entityKind ?? undefined,
        entity_key: missionContext.entityKey ?? undefined,
        label,
        source_watchlist_item_id: missionContext.sourceWatchlistItemId,
        source_run_id: missionContext.sourceRunId,
      });
      setNotice('Mission created and attached to the current operator context.');
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create mission');
    } finally {
      setCreateLoadingType(null);
    }
  };

  const updateMissionStatus = async (missionId: string, status: 'active' | 'paused' | 'archived') => {
    setStatusLoadingId(missionId);
    setNotice(null);
    try {
      await api.investigations.updateMissionStatus(missionId, status);
      setNotice(`Mission moved to ${status}.`);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update mission status');
    } finally {
      setStatusLoadingId(null);
    }
  };

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
        <section>
          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
            Missions
          </p>
          <h1 className="mt-2 font-headline text-4xl font-extrabold tracking-tight" data-testid="mia-missions-heading">
            Operate persistent objectives without losing context
          </h1>
          <p className="mt-3 max-w-3xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
            Missions turn saved watch context into long-lived operator intent. Use them to keep hot launches, builder clusters, recurrence, and proof-qualified reviews alive beyond a single run.
          </p>
        </section>

        {missionContext.entityKind && missionContext.entityKey && (
          <section
            data-testid="mia-missions-context"
            className="rounded-2xl border p-5"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
          >
            <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
              Mission builder context
            </p>
            <p className="mt-2 text-lg font-semibold">
              {missionContext.entityKind === 'builder' ? 'Builder context' : 'Token context'}
            </p>
            <p className="mt-1 text-sm font-semibold" style={{ color: 'var(--on-surface)' }}>
              {missionContext.label ?? 'Saved watch context'}
            </p>
            <p className="mt-1 break-all text-sm" style={{ color: 'var(--on-surface-variant)' }}>
              {missionContext.entityKey}
            </p>
          </section>
        )}

        {notice && (
          <section
            data-testid="mia-missions-notice"
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'rgba(0,255,163,0.08)', borderColor: 'rgba(0,255,163,0.24)', color: 'var(--secondary-container)' }}
          >
            {notice}
          </section>
        )}

        {error && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}
          >
            {error}
          </section>
        )}

        <section
          className="rounded-2xl border p-5"
          style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
        >
          <div className="mb-4">
            <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
              Quick create
            </p>
            <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
              Start with one mission template. You can keep the current watch context attached, then pause or archive it later as the investigation evolves.
            </p>
          </div>

          <div className="grid gap-4 xl:grid-cols-2">
            {MISSION_TEMPLATES.map((template) => (
              <article
                key={template.type}
                className="rounded-xl border p-4"
                style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
              >
                <p className="text-lg font-semibold">{template.label}</p>
                <p className="mt-2 text-sm leading-6" style={{ color: 'var(--on-surface-variant)' }}>
                  {template.description}
                </p>
                <button
                  type="button"
                  data-testid={`mia-mission-template-${template.type}`}
                  onClick={() => createMission(template.type)}
                  disabled={createLoadingType === template.type}
                  className="mt-4 rounded-xl px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50"
                  style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                >
                  {createLoadingType === template.type ? 'Creating...' : 'Create Mission'}
                </button>
              </article>
            ))}
          </div>
        </section>

        {loading && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
          >
            Loading missions...
          </section>
        )}

        {!loading && missions.length === 0 && (
          <section
            className="rounded-xl border p-6 text-sm"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
          >
            No missions yet. Create one from a template above or jump here from a saved watchlist context.
          </section>
        )}

        {missions.length > 0 && (
          <section
            className="rounded-2xl border p-5"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
          >
            <div className="space-y-3">
              {missions.map((mission) => (
                <article
                  key={mission.mission_id}
                  data-testid="mia-mission-row"
                  data-mission-id={mission.mission_id}
                  className="rounded-xl border p-4"
                  style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
                >
                  <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                    <div className="space-y-3">
                      <div className="flex flex-wrap items-center gap-2">
                        <span
                          className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                          style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }}
                        >
                          {missionTypeLabel(mission.mission_type)}
                        </span>
                        <span
                          className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                          style={statusTone(mission.status)}
                        >
                          {mission.status}
                        </span>
                      </div>

                      <div>
                        <p className="text-lg font-semibold">{mission.label}</p>
                        <p className="mt-1 break-all text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                          {mission.entity_key ?? 'Global mission scope'}
                        </p>
                      </div>

                      <div className="grid gap-3 text-sm md:grid-cols-3">
                        <div>
                          <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                            Linked runs
                          </p>
                          <p>{mission.linked_runs_count}</p>
                        </div>
                        <div>
                          <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                            Last update
                          </p>
                          <p>{relativeTime(mission.updated_at)}</p>
                        </div>
                        <div>
                          <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                            Latest run
                          </p>
                          <p>{mission.latest_run_status ?? 'n/a'}</p>
                        </div>
                      </div>
                    </div>

                    <div className="flex flex-wrap gap-3">
                      {mission.latest_run_id && (
                        <Link
                          href={`/mia/runs/${encodeURIComponent(mission.latest_run_id)}`}
                          className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                          style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}
                        >
                          Open Latest Run
                        </Link>
                      )}
                      {mission.status !== 'paused' && mission.status !== 'archived' && (
                        <button
                          type="button"
                          data-testid="mia-mission-pause"
                          onClick={() => updateMissionStatus(mission.mission_id, 'paused')}
                          disabled={statusLoadingId === mission.mission_id}
                          className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50"
                          style={{ borderColor: 'rgba(255,186,73,0.24)', color: 'var(--warning)', background: 'rgba(255,186,73,0.12)' }}
                        >
                          Pause
                        </button>
                      )}
                      {mission.status === 'paused' && (
                        <button
                          type="button"
                          data-testid="mia-mission-resume"
                          onClick={() => updateMissionStatus(mission.mission_id, 'active')}
                          disabled={statusLoadingId === mission.mission_id}
                          className="rounded-xl px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50"
                          style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                        >
                          Resume
                        </button>
                      )}
                      {mission.status !== 'archived' && (
                        <button
                          type="button"
                          data-testid="mia-mission-archive"
                          onClick={() => updateMissionStatus(mission.mission_id, 'archived')}
                          disabled={statusLoadingId === mission.mission_id}
                          className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50"
                          style={{ borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)', background: 'rgba(255,107,107,0.08)' }}
                        >
                          Archive
                        </button>
                      )}
                    </div>
                  </div>
                </article>
              ))}
            </div>
          </section>
        )}
      </main>
    </>
  );
}
