'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import {
  FaBrain,
  FaMagnifyingGlass,
  FaMoneyBillWave,
  FaSitemap,
  FaUserSecret,
  FaWandMagicSparkles,
} from 'react-icons/fa6';
import { api } from '@/lib/api';
import { buildLinkedLaunchEvidenceView } from '@/lib/deep-research';
import type {
  DeepResearchPreviewResponse,
  DeepResearchReportResponse,
  DeepResearchRunResponse,
  DeepResearchRunStage,
  DeepResearchRunStatus,
  DeepResearchRunTraceResponse,
  DeepResearchStatusResponse,
} from '@/lib/types';

interface PatternAnalogView {
  token_address: string;
  window_end: string;
  match_score: number;
  match_label: string;
  outcome_class: string;
  rationale: string;
  notable_differences: string[];
}

interface PatternHorizonView {
  horizon_hours: number;
  match_label: string;
  outcome_class: string;
  confidence: number;
  expected_path_summary: string;
  rationale: string;
  analogs: PatternAnalogView[];
}

interface DeepResearchPanelsProps {
  tokenAddress: string;
  preview: DeepResearchPreviewResponse | null;
  status: DeepResearchStatusResponse | null;
  report: DeepResearchReportResponse | null;
  entitlementToken: string | null;
  loading: boolean;
  error: string | null;
}

function shortAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return 'n/a';
  return new Date(value).toLocaleString();
}

function formatUsdCents(value: number) {
  return `$${(value / 100).toFixed(2)}`;
}

function formatPercent(value: number) {
  return `${(value * 100).toFixed(0)}%`;
}

function formatLatency(value: number | null) {
  if (value === null) return 'n/a';
  if (value < 1000) return `${value} ms`;
  return `${(value / 1000).toFixed(1)} s`;
}

function stageLabel(stage: DeepResearchRunStage) {
  switch (stage) {
    case 'plan':
      return 'Plan';
    case 'gather_internal':
      return 'Gather Internal';
    case 'gather_external':
      return 'Gather External';
    case 'synthesize':
      return 'Synthesize';
    case 'finalize':
      return 'Finalize';
    default:
      return stage;
  }
}

function statusTone(status: DeepResearchRunStatus): 'safe' | 'warn' | 'primary' | 'danger' {
  switch (status) {
    case 'completed':
      return 'safe';
    case 'failed':
      return 'danger';
    case 'queued':
    case 'running':
      return 'primary';
    case 'skipped':
      return 'warn';
    default:
      return 'primary';
  }
}

function StatusPill({
  label,
  tone,
}: {
  label: string;
  tone: 'safe' | 'warn' | 'primary' | 'danger';
}) {
  const style =
    tone === 'safe'
      ? {
          color: 'var(--secondary-container)',
          background: 'rgba(0,255,163,0.08)',
          borderColor: 'rgba(0,255,163,0.16)',
        }
      : tone === 'primary'
        ? {
            color: 'var(--primary)',
            background: 'rgba(105,137,255,0.08)',
            borderColor: 'rgba(105,137,255,0.16)',
          }
        : tone === 'danger'
          ? {
              color: 'var(--danger)',
              background: 'rgba(255,107,107,0.1)',
              borderColor: 'rgba(255,107,107,0.16)',
            }
          : {
              color: 'var(--warning)',
              background: 'rgba(255,186,73,0.12)',
              borderColor: 'rgba(255,186,73,0.18)',
            };

  return (
    <span className="rounded-full border px-3 py-1 text-[11px] uppercase tracking-[0.16em]" style={style}>
      {label}
    </span>
  );
}

function PremiumPanel({
  title,
  icon,
  children,
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <section
      className="rounded-[1.75rem] border p-4 shadow-[0_26px_60px_rgba(8,12,24,0.28)] md:p-6"
      style={{
        background:
          'linear-gradient(180deg, rgba(255,255,255,0.025), rgba(255,255,255,0.01)), var(--surface-container-low)',
        borderColor: 'rgba(148,160,194,0.16)',
      }}
    >
      <div className="mb-4 flex items-center gap-2">
        <span style={{ color: 'var(--primary)' }}>{icon}</span>
        <h2 className="font-headline text-lg font-bold tracking-tight">{title}</h2>
      </div>
      {children}
    </section>
  );
}

function EvidenceList({
  title,
  items,
  empty,
}: {
  title: string;
  items: string[];
  empty: string;
}) {
  return (
    <div>
      <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {title}
      </p>
      {items.length > 0 ? (
        <div className="space-y-2 text-xs">
          {items.map((item) => (
            <div
              key={item}
              className="rounded-lg px-3 py-2"
              style={{ color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }}
            >
              {item}
            </div>
          ))}
        </div>
      ) : (
        <p className="text-xs" style={{ color: 'var(--on-surface-variant)' }}>
          {empty}
        </p>
      )}
    </div>
  );
}

function runStorageKey(tokenAddress: string) {
  return `mia:deep-research-run:${tokenAddress.toLowerCase()}`;
}

function isPatternHorizon(value: unknown): value is PatternHorizonView {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.horizon_hours === 'number' &&
    typeof candidate.match_label === 'string' &&
    typeof candidate.outcome_class === 'string'
  );
}

function patternHorizonDetails(section: DeepResearchReportResponse['sections'][number]): PatternHorizonView[] {
  const horizons = section.details && Array.isArray(section.details.horizons)
    ? section.details.horizons.filter(isPatternHorizon)
    : [];
  return horizons.sort((left, right) => left.horizon_hours - right.horizon_hours);
}

export function DeepResearchPanels({
  tokenAddress,
  preview,
  status,
  report,
  entitlementToken,
  loading,
  error,
}: DeepResearchPanelsProps) {
  const linkedLaunchEvidence = buildLinkedLaunchEvidenceView(report);
  const premiumReady = status?.premium_state === 'report_ready';
  const runAccessEnabled = Boolean(tokenAddress && entitlementToken);
  const [run, setRun] = useState<DeepResearchRunResponse | null>(null);
  const [trace, setTrace] = useState<DeepResearchRunTraceResponse | null>(null);
  const [workspaceReport, setWorkspaceReport] = useState<DeepResearchReportResponse | null>(null);
  const [runLoading, setRunLoading] = useState(false);
  const [runError, setRunError] = useState<string | null>(null);

  const activeReport = workspaceReport ?? report;
  const toolLedger = useMemo(() => trace?.tool_calls ?? [], [trace]);
  const paymentLedger = useMemo(() => trace?.payment_ledger ?? [], [trace]);
  const stepTimeline = useMemo(() => trace?.steps ?? [], [trace]);
  const planningSummary = useMemo(
    () => stepTimeline.find((step) => step.step_key === 'plan')?.summary ?? null,
    [stepTimeline]
  );
  const exactBudgetDisplay = useMemo(() => {
    if (paymentLedger.length === 0) return null;
    const parsed = paymentLedger
      .map((entry) => {
        const [amount, asset] = entry.amount_display.split(/\s+/);
        return { amount, asset: asset ?? entry.asset };
      })
      .filter((entry) => entry.amount);
    if (parsed.length === 0) return null;
    const asset = parsed[0]?.asset ?? 'USDC';
    const total = parsed.reduce((sum, entry) => sum + Number(entry.amount), 0);
    return `${total.toFixed(6)} ${asset}`;
  }, [paymentLedger]);

  useEffect(() => {
    setRun(null);
    setTrace(null);
    setWorkspaceReport(null);
    setRunError(null);
  }, [tokenAddress]);

  useEffect(() => {
    if (!runAccessEnabled) return;
    if (typeof window === 'undefined') return;

    const storedRunId = window.localStorage.getItem(runStorageKey(tokenAddress));
    if (!storedRunId) return;

    let cancelled = false;

    const restoreRun = async () => {
      try {
        const [runState, traceState] = await Promise.all([
          api.tokens.deepResearchRun(tokenAddress, storedRunId, entitlementToken),
          api.tokens.deepResearchRunTrace(tokenAddress, storedRunId, entitlementToken),
        ]);

        if (cancelled) return;
        setRun(runState);
        setTrace(traceState);

        if (runState.report_ready) {
          const premium = await api.tokens.deepResearchRunReport(
            tokenAddress,
            storedRunId,
            entitlementToken
          );
          if (!cancelled) {
            setWorkspaceReport(premium.data);
          }
        }
      } catch {
        if (cancelled) return;
        window.localStorage.removeItem(runStorageKey(tokenAddress));
      }
    };

    void restoreRun();

    return () => {
      cancelled = true;
    };
  }, [tokenAddress, entitlementToken, runAccessEnabled]);

  useEffect(() => {
    if (!runAccessEnabled) return;
    if (!run) return;
    if (run.status !== 'queued' && run.status !== 'running') return;

    const timer = window.setInterval(async () => {
      try {
        const [nextRun, nextTrace] = await Promise.all([
          api.tokens.deepResearchRun(tokenAddress, run.run_id, entitlementToken),
          api.tokens.deepResearchRunTrace(tokenAddress, run.run_id, entitlementToken),
        ]);
        setRun(nextRun);
        setTrace(nextTrace);

        if (nextRun.report_ready) {
          const premium = await api.tokens.deepResearchRunReport(
            tokenAddress,
            run.run_id,
            entitlementToken
          );
          setWorkspaceReport(premium.data);
        }
      } catch (pollError) {
        setRunError(
          pollError instanceof Error
            ? pollError.message
            : 'Deep Research trace polling failed.'
        );
      }
    }, 2000);

    return () => window.clearInterval(timer);
  }, [entitlementToken, run, runAccessEnabled, tokenAddress]);

  const startResearchRun = async () => {
    if (!runAccessEnabled || !entitlementToken) {
      setRunError('An active Deep Research entitlement is required before starting a run.');
      return;
    }

    setRunLoading(true);
    setRunError(null);

    try {
      const createdRun = await api.tokens.deepResearchCreateRun(tokenAddress, entitlementToken);
      if (typeof window !== 'undefined') {
        window.localStorage.setItem(runStorageKey(tokenAddress), createdRun.run_id);
      }
      setRun(createdRun);

      const traceState = await api.tokens.deepResearchRunTrace(
        tokenAddress,
        createdRun.run_id,
        entitlementToken
      );
      setTrace(traceState);

      if (createdRun.report_ready) {
        const premium = await api.tokens.deepResearchRunReport(
          tokenAddress,
          createdRun.run_id,
          entitlementToken
        );
        setWorkspaceReport(premium.data);
      }
    } catch (createError) {
      setRunError(
        createError instanceof Error
          ? createError.message
          : 'Deep Research run could not be created.'
      );
    } finally {
      setRunLoading(false);
    }
  };

  return (
    <section id="deep-research" className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
      <PremiumPanel title="Deep Research Workspace" icon={<FaMagnifyingGlass size={14} />}>
        {preview ? (
          <>
            <div
              className="rounded-2xl border p-4 md:p-5"
              style={{
                background:
                  'linear-gradient(135deg, rgba(105,137,255,0.16), rgba(9,14,25,0.96))',
                borderColor: 'rgba(111,141,255,0.18)',
              }}
            >
              <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
                <div className="max-w-2xl">
                  <div className="flex flex-wrap items-center gap-2">
                    <StatusPill label={preview.provider_path} tone="primary" />
                    <StatusPill
                      label={premiumReady ? 'report ready' : status?.premium_state ?? 'premium lane'}
                      tone={premiumReady ? 'safe' : 'warn'}
                    />
                    {run && <StatusPill label={`run ${run.status}`} tone={statusTone(run.status)} />}
                  </div>
                  <p className="mt-4 text-base font-semibold md:text-lg">
                    Run a planner-led premium evidence workspace with traceable steps, tool calls, and the full dossier in one place.
                  </p>
                  <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                    This lane is built for depth: internal launch intelligence first, optional external enrichment second, and a visible execution trace so the run never hides how the evidence was gathered. MIA explains the data, but the final call stays with the user.
                  </p>
                </div>

                <div
                  className="min-w-[17rem] rounded-2xl border p-4 shadow-[0_18px_36px_rgba(79,110,255,0.18)]"
                  style={{
                    background: premiumReady
                      ? 'rgba(0,255,163,0.08)'
                      : 'rgba(255,255,255,0.035)',
                    borderColor: premiumReady
                      ? 'rgba(0,255,163,0.16)'
                      : 'rgba(148,160,194,0.18)',
                  }}
                >
                  <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                    Workspace state
                  </p>
                  <p className="mt-2 text-base font-semibold">
                    {run
                      ? `Run ${run.status.replace('_', ' ')}`
                      : premiumReady
                        ? 'Premium report available'
                        : preview.unlock_cta}
                  </p>
                  <p className="mt-2 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
                    {run
                      ? `Current phase: ${stageLabel(run.current_phase)}.`
                      : runAccessEnabled
                        ? 'Start a research run to expose the planner trace, tool ledger, and the evidence dossier.'
                        : 'Unlock or attach an entitlement first. The run-based workspace only opens for an active premium session.'}
                  </p>
                  <button
                    type="button"
                    data-testid="deep-research-start-run"
                    onClick={startResearchRun}
                    disabled={!runAccessEnabled || runLoading}
                    className="mt-4 inline-flex w-full items-center justify-center rounded-xl px-4 py-3 text-sm font-semibold transition disabled:cursor-not-allowed disabled:opacity-50"
                    style={{
                      background: 'linear-gradient(135deg, var(--primary), rgba(143,168,255,0.98))',
                      color: 'var(--on-primary)',
                      boxShadow: '0 18px 34px rgba(105,137,255,0.24)',
                    }}
                  >
                    {runLoading ? 'Starting run…' : run ? 'Run another research pass' : 'Start Deep Research Run'}
                  </button>
                </div>
              </div>
            </div>

            <div className="mt-5 grid gap-3 md:grid-cols-2">
              {preview.sections.map((section) => (
                <div
                  key={section.id}
                  className="rounded-xl border p-4"
                  style={{ background: 'rgba(255,255,255,0.025)', borderColor: 'rgba(148,160,194,0.14)' }}
                >
                  <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                    {section.stage.replace('_', ' ')}
                  </p>
                  <p className="mt-2 font-semibold">{section.title}</p>
                  <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                    {section.summary}
                  </p>
                </div>
              ))}
            </div>

            {(loading || runLoading) && (
              <p className="mt-4 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                Preparing the Deep Research workspace...
              </p>
            )}

            {(error || runError) && (
              <p className="mt-4 text-sm" style={{ color: 'var(--warning)' }}>
                {runError ?? error}
              </p>
            )}

            {run && (
              <div
                data-testid="deep-research-run-header"
                className="mt-5 rounded-2xl border p-4 md:p-5"
                style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
              >
                <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
                  <div>
                    <div className="flex flex-wrap items-center gap-2">
                      <StatusPill label={`run ${run.status}`} tone={statusTone(run.status)} />
                      <StatusPill label={stageLabel(run.current_phase)} tone="primary" />
                      <StatusPill
                        label={run.report_ready ? 'dossier attached' : 'trace building'}
                        tone={run.report_ready ? 'safe' : 'warn'}
                      />
                    </div>
                    <p className="mt-3 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                      {planningSummary ??
                        'The planner has already set the path for this run. The workspace below shows each step, which tools ran, and what evidence was attached before the dossier was assembled.'}
                    </p>
                  </div>

                  <div className="grid gap-3 sm:grid-cols-2 xl:min-w-[20rem]">
                    <div className="rounded-xl border px-4 py-3" style={{ borderColor: 'rgba(148,160,194,0.14)' }}>
                      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                        Run ID
                      </p>
                      <p className="mt-2 font-mono text-xs">{shortAddress(run.run_id, 10, 6)}</p>
                    </div>
                    <div className="rounded-xl border px-4 py-3" style={{ borderColor: 'rgba(148,160,194,0.14)' }}>
                      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                        Budget used
                      </p>
                      <p className="mt-2 text-sm font-semibold">
                        {exactBudgetDisplay ?? formatUsdCents(run.budget_usage_cents)}
                      </p>
                    </div>
                    <div className="rounded-xl border px-4 py-3" style={{ borderColor: 'rgba(148,160,194,0.14)' }}>
                      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                        Paid calls
                      </p>
                      <p className="mt-2 text-sm font-semibold">{run.paid_calls_count}</p>
                    </div>
                    <div className="rounded-xl border px-4 py-3" style={{ borderColor: 'rgba(148,160,194,0.14)' }}>
                      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                        Created
                      </p>
                      <p className="mt-2 text-xs">{formatDateTime(run.created_at)}</p>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {trace ? (
              <div className="mt-5 grid gap-5 xl:grid-cols-[1.05fr_0.95fr]">
                <div
                  data-testid="deep-research-run-trace"
                  className="rounded-2xl border p-4 md:p-5"
                  style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
                >
                  <div className="mb-4 flex items-center gap-2">
                    <FaSitemap size={13} style={{ color: 'var(--primary)' }} />
                    <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                      Research trace
                    </p>
                  </div>

                  <div className="space-y-3">
                    {stepTimeline.map((step) => (
                      <div
                        key={step.id}
                        className="rounded-xl border p-4"
                        style={{ background: 'rgba(255,255,255,0.02)', borderColor: 'rgba(148,160,194,0.12)' }}
                      >
                        <div className="flex flex-wrap items-center gap-2">
                          <StatusPill label={step.title} tone="primary" />
                          <StatusPill label={step.status} tone={statusTone(step.status)} />
                          {step.agent_name && <StatusPill label={step.agent_name} tone="warn" />}
                        </div>
                        {step.summary && (
                          <p className="mt-3 text-sm leading-7">{step.summary}</p>
                        )}
                        {step.tool_name && (
                          <p className="mt-2 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                            Tool lane: {step.tool_name}
                          </p>
                        )}
                        {step.evidence.length > 0 && (
                          <div className="mt-3 space-y-2">
                            {step.evidence.map((item) => (
                              <div
                                key={`${step.id}-${item}`}
                                className="rounded-lg px-3 py-2 text-xs"
                                style={{ background: 'rgba(105,137,255,0.08)', color: 'var(--on-surface)' }}
                              >
                                {item}
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>

                <div
                  data-testid="deep-research-tool-ledger"
                  className="rounded-2xl border p-4 md:p-5"
                  style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
                >
                  <div className="mb-4 flex items-center gap-2">
                    <FaMoneyBillWave size={13} style={{ color: 'var(--primary)' }} />
                    <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                      Tool ledger
                    </p>
                  </div>

                  {toolLedger.length > 0 ? (
                    <div className="space-y-3">
                      {toolLedger.map((call) => {
                        const payment = paymentLedger.find((entry) => entry.tool_call_id === call.id);
                        return (
                        <div
                          key={call.id}
                          className="rounded-xl border p-4"
                          style={{ background: 'rgba(255,255,255,0.02)', borderColor: 'rgba(148,160,194,0.12)' }}
                        >
                          <div className="flex flex-wrap items-center gap-2">
                            <StatusPill label={call.tool_name.replaceAll('_', ' ')} tone="primary" />
                            <StatusPill label={call.status} tone={statusTone(call.status)} />
                            {call.provider && <StatusPill label={call.provider} tone="warn" />}
                          </div>
                          {call.summary && (
                            <p className="mt-3 text-sm leading-7">{call.summary}</p>
                          )}
                          <div className="mt-3 grid gap-2 sm:grid-cols-2">
                            <div className="rounded-lg px-3 py-2 text-xs" style={{ background: 'var(--surface-container-lowest)' }}>
                              <span style={{ color: 'var(--outline)' }}>Latency:</span> {formatLatency(call.latency_ms)}
                            </div>
                            <div className="rounded-lg px-3 py-2 text-xs" style={{ background: 'var(--surface-container-lowest)' }}>
                              <span style={{ color: 'var(--outline)' }}>Cost:</span>{' '}
                              {payment?.amount_display ?? formatUsdCents(call.cost_cents)}
                            </div>
                          </div>
                          {payment && (
                            <div className="mt-3 rounded-lg px-3 py-2 text-xs" style={{ background: 'rgba(0,255,163,0.08)' }}>
                              <span style={{ color: 'var(--outline)' }}>Payment:</span> {payment.amount_display} on{' '}
                              {payment.network}
                              {payment.tx_hash && (
                                <>
                                  {' · '}
                                  <span className="font-mono">{shortAddress(payment.tx_hash, 10, 6)}</span>
                                </>
                              )}
                            </div>
                          )}
                          {call.evidence.length > 0 && (
                            <div className="mt-3 space-y-2">
                              {call.evidence.map((item) => (
                                <div
                                  key={`${call.id}-${item}`}
                                  className="rounded-lg px-3 py-2 text-xs"
                                  style={{ background: 'rgba(0,255,163,0.08)', color: 'var(--secondary-container)' }}
                                >
                                  {item}
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                        );
                      })}
                    </div>
                  ) : (
                    <p className="text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                      Start a research run to see every internal tool call land in the premium ledger.
                    </p>
                  )}
                </div>
              </div>
            ) : (
              <div
                className="mt-5 rounded-2xl border p-4 text-sm leading-7"
                style={{
                  background: 'rgba(255,255,255,0.03)',
                  borderColor: 'rgba(141,144,161,0.12)',
                  color: 'var(--on-surface-variant)',
                }}
              >
                {runAccessEnabled
                  ? 'No research run has been attached yet. Start one to expose the planner steps, tool ledger, and final premium dossier inside this workspace.'
                  : 'The run-based workspace becomes available after premium unlock because the trace and dossier endpoints require an active entitlement.'}
              </div>
            )}

            {activeReport && (
              <div className="mt-5 space-y-3">
                <div
                  className="rounded-2xl border p-4 md:p-5"
                  style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}
                >
                  <div className="mb-4 flex items-center gap-2">
                    <FaBrain size={13} style={{ color: 'var(--primary)' }} />
                    <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                      Evidence dossier
                    </p>
                  </div>
                  <p className="text-sm leading-7">{activeReport.executive_summary}</p>
                </div>

                {activeReport.sections
                  .filter((section) => section.id !== 'linked-launch-cluster')
                  .map((section) => (
                    <div
                      key={section.id}
                      className="rounded-xl border p-4"
                      style={{ background: 'rgba(255,255,255,0.04)', borderColor: 'rgba(148,160,194,0.14)' }}
                    >
                      <div className="flex flex-wrap items-center gap-2 text-[11px] uppercase tracking-widest">
                        <StatusPill label={section.stage.replaceAll('_', ' ')} tone="warn" />
                        {section.provider && <StatusPill label={section.provider} tone="primary" />}
                        {section.fallback_note && <StatusPill label="fallback noted" tone="warn" />}
                        {section.confidence && (
                          <StatusPill label={`confidence ${section.confidence}`} tone="safe" />
                        )}
                      </div>
                      <p className="mt-3 font-semibold">{section.title}</p>
                      <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                        {section.summary}
                      </p>
                      {section.evidence && section.evidence.length > 0 && (
                        <div className="mt-3">
                          <EvidenceList title="Evidence" items={section.evidence} empty="No evidence lines attached." />
                        </div>
                      )}
                      {section.id === 'pattern-match-engine' && patternHorizonDetails(section).length > 0 && (
                        <div className="mt-4 grid gap-3 lg:grid-cols-3">
                          {patternHorizonDetails(section).map((horizon) => (
                            <div
                              key={`${section.id}-${horizon.horizon_hours}`}
                              className="rounded-xl border p-4"
                              style={{
                                background: 'rgba(255,255,255,0.025)',
                                borderColor: 'rgba(148,160,194,0.14)',
                              }}
                            >
                              <div className="flex flex-wrap items-center gap-2">
                                <StatusPill label={`${horizon.horizon_hours}H`} tone="primary" />
                                <StatusPill label={horizon.match_label.replaceAll('_', ' ')} tone="warn" />
                                <StatusPill label={formatPercent(horizon.confidence)} tone="safe" />
                              </div>
                              <p className="mt-3 text-sm font-semibold">
                                {horizon.outcome_class.replaceAll('_', ' ')}
                              </p>
                              <p className="mt-2 text-sm leading-6" style={{ color: 'var(--on-surface-variant)' }}>
                                {horizon.expected_path_summary}
                              </p>
                              <p className="mt-3 text-xs leading-6" style={{ color: 'var(--outline)' }}>
                                {horizon.rationale}
                              </p>
                              {horizon.analogs.length > 0 && (
                                <div className="mt-3 space-y-2">
                                  {horizon.analogs.slice(0, 2).map((analog) => (
                                    <div
                                      key={`${horizon.horizon_hours}-${analog.token_address}-${analog.window_end}`}
                                      className="rounded-lg px-3 py-2 text-xs"
                                      style={{ background: 'rgba(105,137,255,0.08)' }}
                                    >
                                      <div className="flex items-center justify-between gap-3">
                                        <span className="font-mono">{shortAddress(analog.token_address, 8, 4)}</span>
                                        <span>{formatPercent(analog.match_score)}</span>
                                      </div>
                                      <p className="mt-1" style={{ color: 'var(--on-surface-variant)' }}>
                                        {analog.outcome_class.replaceAll('_', ' ')}
                                      </p>
                                    </div>
                                  ))}
                                </div>
                              )}
                            </div>
                          ))}
                        </div>
                      )}
                      <div className="mt-3 flex flex-wrap gap-3 text-xs">
                        {section.source_url && (
                          <a href={section.source_url} target="_blank" rel="noreferrer" style={{ color: 'var(--primary)' }}>
                            Open source
                          </a>
                        )}
                        {section.observed_at && (
                          <span style={{ color: 'var(--on-surface-variant)' }}>
                            observed: {section.observed_at}
                          </span>
                        )}
                      </div>
                    </div>
                  ))}
              </div>
            )}
          </>
        ) : (
          <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
            Deep Research preview has not loaded yet.
          </p>
        )}
      </PremiumPanel>

      <div className="space-y-6">
        <PremiumPanel title="Workspace Notes" icon={<FaWandMagicSparkles size={14} />}>
          <div className="space-y-4">
            <div
              className="rounded-xl border p-4"
              style={{ background: 'rgba(105,137,255,0.05)', borderColor: 'rgba(105,137,255,0.12)' }}
            >
              <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                What phase 3 adds
              </p>
              <p className="mt-2 text-sm leading-7">
                The premium lane now has a run header, planner trace, tool ledger, and evidence dossier in one place. The goal is not to push a verdict, but to show how MIA assembled the data and where each layer came from.
              </p>
            </div>

            <div
              className="rounded-xl border p-4"
              style={{ background: 'rgba(255,255,255,0.025)', borderColor: 'rgba(148,160,194,0.14)' }}
            >
              <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                Current limits
              </p>
              <div className="mt-3 space-y-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                <p>Planner steps and internal tool ledger are live.</p>
                <p>Paid upstream Heurist lanes now attach to the external step and land in the payment ledger.</p>
                <p>ML pattern comparison at 1H, 6H, and 24H is live as supporting historical context.</p>
                <p>Unlock is still required before the run workspace can pull trace or dossier data.</p>
              </div>
            </div>
          </div>
        </PremiumPanel>

        <PremiumPanel title="Linked Launch Cluster" icon={<FaUserSecret size={14} />}>
          {linkedLaunchEvidence ? (
            <>
              <div
                className="rounded-xl border p-4"
                style={{ background: 'rgba(0,255,163,0.06)', borderColor: 'rgba(0,255,163,0.14)' }}
              >
                <div className="flex flex-wrap items-center gap-2 text-[11px] uppercase tracking-widest">
                  <StatusPill
                    label={`confidence ${linkedLaunchEvidence.confidence}`}
                    tone={
                      linkedLaunchEvidence.confidence === 'high'
                        ? 'safe'
                        : linkedLaunchEvidence.confidence === 'medium'
                          ? 'primary'
                          : 'warn'
                    }
                  />
                  <StatusPill label="MIA internal linking" tone="primary" />
                </div>
                <p className="mt-3 text-base font-semibold">{linkedLaunchEvidence.summary}</p>
              </div>

              <div className="mt-5">
                <EvidenceList
                  title="Evidence"
                  items={linkedLaunchEvidence.evidence}
                  empty="No linking evidence returned."
                />
              </div>

              <div className="mt-5 grid gap-4 md:grid-cols-2">
                <div>
                  <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                    Repeated wallets
                  </p>
                  {linkedLaunchEvidence.repeatedWallets.length > 0 ? (
                    <div className="space-y-2">
                      {linkedLaunchEvidence.repeatedWallets.slice(0, 5).map((wallet) => (
                        <div
                          key={wallet}
                          className="rounded-lg border px-3 py-2 text-xs font-mono"
                          style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}
                        >
                          {shortAddress(wallet, 10, 6)}
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                      No repeated-wallet overlap recovered yet.
                    </p>
                  )}
                </div>

                <div>
                  <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                    Prior launches
                  </p>
                  {linkedLaunchEvidence.relatedTokens.length > 0 ? (
                    <div className="space-y-2">
                      {linkedLaunchEvidence.relatedTokens.map((token) => (
                        <Link
                          key={token.contract_address}
                          href={`/mia?q=${encodeURIComponent(token.contract_address)}`}
                          className="block rounded-lg border px-3 py-3 text-sm transition-colors hover:border-[rgba(105,137,255,0.35)]"
                          style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}
                        >
                          <div className="flex items-center justify-between gap-3">
                            <div>
                              <p className="font-semibold">
                                {token.symbol ?? token.name ?? shortAddress(token.contract_address)}
                              </p>
                              <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                                {token.name ?? shortAddress(token.contract_address, 10, 6)}
                              </p>
                            </div>
                            <div className="flex flex-col items-end gap-1 text-[10px] uppercase tracking-[0.16em]">
                              <span style={{ color: token.is_rug ? 'var(--danger)' : 'var(--secondary-container)' }}>
                                {token.is_rug ? 'rug' : token.graduated ? 'graduated' : 'tracked'}
                              </span>
                            </div>
                          </div>
                        </Link>
                      ))}
                    </div>
                  ) : (
                    <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                      No prior launches were linked in the cached premium report.
                    </p>
                  )}
                </div>
              </div>
            </>
          ) : (
            <div
              className="rounded-xl border p-4 text-sm"
              style={{
                background: 'var(--surface-container-lowest)',
                borderColor: 'rgba(141,144,161,0.12)',
                color: 'var(--on-surface-variant)',
              }}
            >
              {status?.premium_state === 'report_ready'
                ? 'The premium report is ready, but no linked-launch evidence section was attached.'
                : 'Unlock the premium dossier to expose MIA’s linked-launch cluster analysis and repeated-wallet evidence.'}
            </div>
          )}
        </PremiumPanel>
      </div>
    </section>
  );
}
