'use client';

import Link from 'next/link';
import { useEffect, useState } from 'react';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { InvestigationWatchlistItemResponse } from '@/lib/types';

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

function statusTone(status: string | null | undefined) {
  if (status === 'completed') {
    return { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' };
  }
  if (status === 'watching' || status === 'escalated') {
    return { color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' };
  }
  if (status === 'archived') {
    return { color: 'var(--outline)', background: 'rgba(255,255,255,0.05)' };
  }
  return { color: 'var(--warning)', background: 'rgba(255,186,73,0.12)' };
}

function entityKindLabel(value: string) {
  return value === 'builder' ? 'Builder watch' : 'Token watch';
}

export default function MiaWatchlistPage() {
  const [items, setItems] = useState<InvestigationWatchlistItemResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deleteLoadingId, setDeleteLoadingId] = useState<string | null>(null);

  const load = async () => {
    try {
      const response = await api.investigations.watchlist();
      setItems(response.data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load watchlist');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const removeItem = async (itemId: string) => {
    setDeleteLoadingId(itemId);
    try {
      await api.investigations.deleteWatchlistItem(itemId);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to remove watchlist item');
    } finally {
      setDeleteLoadingId(null);
    }
  };

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
        <section>
          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--primary)' }}>
            Watch
          </p>
          <h1 className="mt-2 font-headline text-4xl font-extrabold tracking-tight" data-testid="mia-watchlist-heading">
            Monitor saved tokens and builders with continuity
          </h1>
          <p className="mt-3 max-w-3xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
            Keep persistent watch context here so future runs can be read as ongoing monitoring instead of getting lost in the general run console.
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

        {loading && (
          <section
            className="rounded-xl border p-4 text-sm"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
          >
            Loading watchlist...
          </section>
        )}

        {!loading && items.length === 0 && !error && (
          <section
            className="rounded-xl border p-6 text-sm"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
          >
            No watches saved yet. Open an investigation and save a token or builder to give future runs a persistent operator context.
          </section>
        )}

        {items.length > 0 && (
          <section
            className="rounded-2xl border p-5"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)' }}
          >
            <div className="space-y-3">
              {items.map((item) => {
                const watchHref =
                  item.entity_kind === 'builder'
                    ? `/deployer/${encodeURIComponent(item.entity_key)}`
                    : `/mia?q=${encodeURIComponent(item.entity_key)}`;

                return (
                  <article
                    key={item.item_id}
                    data-testid="mia-watchlist-row"
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
                            {entityKindLabel(item.entity_kind)}
                          </span>
                          {item.latest_run_status && (
                            <span
                              data-testid="mia-watchlist-run-status"
                              className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                              style={statusTone(item.latest_run_status)}
                            >
                              {item.latest_run_status}
                            </span>
                          )}
                        </div>

                        <div>
                          <p className="text-lg font-semibold" data-testid="mia-watchlist-label">
                            {item.label}
                          </p>
                          <p className="mt-1 break-all text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                            {item.entity_key}
                          </p>
                        </div>

                        <div className="grid gap-3 text-sm md:grid-cols-3">
                          <div>
                            <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                              Linked runs
                            </p>
                            <p data-testid="mia-watchlist-linked-runs">{item.linked_runs_count}</p>
                          </div>
                          <div>
                            <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                              Latest run update
                            </p>
                            <p>{formatRelativeTime(item.latest_run_updated_at)}</p>
                          </div>
                          <div>
                            <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
                              Source run
                            </p>
                            <p>{item.source_run_id ? item.source_run_id.slice(0, 8) : 'manual save'}</p>
                          </div>
                        </div>
                      </div>

                      <div className="flex flex-wrap gap-3">
                        <Link
                          href={watchHref}
                          className="rounded-xl px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                          style={{ background: 'var(--primary-container)', color: 'var(--on-primary-container)' }}
                        >
                          {item.entity_kind === 'builder' ? 'Open Builder' : 'Open Investigation'}
                        </Link>
                        <Link
                          href={`/mia/missions?entity_kind=${encodeURIComponent(item.entity_kind)}&entity_key=${encodeURIComponent(item.entity_key)}&label=${encodeURIComponent(item.label)}${item.source_run_id ? `&source_run_id=${encodeURIComponent(item.source_run_id)}` : ''}&source_watchlist_item_id=${encodeURIComponent(item.item_id)}`}
                          data-testid="mia-watchlist-open-missions"
                          className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                          style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }}
                        >
                          Open Mission Builder
                        </Link>
                        {item.latest_run_id && (
                          <Link
                            href={`/mia/runs/${encodeURIComponent(item.latest_run_id)}`}
                            className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
                            style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--primary)', background: 'var(--surface-container-lowest)' }}
                          >
                            Open Latest Run
                          </Link>
                        )}
                        <button
                          type="button"
                          data-testid="mia-watchlist-remove"
                          onClick={() => removeItem(item.item_id)}
                          disabled={deleteLoadingId === item.item_id}
                          className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50"
                          style={{ borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)', background: 'rgba(255,107,107,0.08)' }}
                        >
                          {deleteLoadingId === item.item_id ? 'Removing...' : 'Remove'}
                        </button>
                      </div>
                    </div>
                  </article>
                );
              })}
            </div>
          </section>
        )}
      </main>
    </>
  );
}
