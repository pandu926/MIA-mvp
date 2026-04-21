'use client';

import Link from 'next/link';
import { useRouter, useSearchParams } from 'next/navigation';
import { Suspense, useCallback, useEffect, useMemo, useState } from 'react';
import {
  FaBolt,
  FaBrain,
  FaChartLine,
  FaDatabase,
  FaGlobe,
  FaLink,
  FaMagnifyingGlass,
  FaShieldHalved,
  FaTriangleExclamation,
  FaUserSecret,
  FaWaveSquare,
} from 'react-icons/fa6';
import { DeepResearchPanels } from '@/components/mia/DeepResearchPanels';
import { AskMiaEntryCard } from '@/components/mia/AskMiaEntryCard';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import {
  buildMiaActionBrief,
  buildMiaExecutionPlan,
  buildMiaProofSnapshot,
  buildRecentOutcomeCards,
  formatOutcomeMove,
  type MiaExecutionPlan,
  type MiaOutcomeCard,
} from '@/lib/mia-product';
import type {
  DecisionSubscoreResponse,
  DeepResearchPreviewResponse,
  DeepResearchReportResponse,
  DeepResearchStatusResponse,
  InvestigationRunSummary,
  InvestigationRunDetailResponse,
  InvestigationResponse,
  TokenSummary,
} from '@/lib/types';

type MiaViewMode = 'quick' | 'full';
type MiaReadMode = 'human' | 'analyst';
type MiaWorkspaceSection = 'overview' | 'layers' | 'timeline' | 'proof' | 'sources' | 'tools';
type MiaLayerCard = {
  id: string;
  title: string;
  kicker: string;
  summary: string;
  humanText: string;
  analystPoints: string[];
  meter: number;
  tone: 'safe' | 'primary' | 'warn' | 'danger';
};

type ResolvedDecisionScorecard = {
  decision_score: number | null;
  verdict: string;
  confidence_label: string;
  primary_reason: string;
  primary_risk: string;
  subscores: DecisionSubscoreResponse[];
};

function looksLikeAddress(value: string) {
  return /^0x[a-fA-F0-9]{40}$/.test(value.trim());
}

function shortAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

function statusTone(value: string) {
  const lower = value.toLowerCase();
  if (lower.includes('high conviction') || lower.includes('high') || lower.includes('safe') || lower.includes('trusted')) {
    return { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.12)' };
  }
  if (lower.includes('avoid') || lower.includes('danger') || lower.includes('underperform')) {
    return { color: 'var(--danger)', background: 'rgba(255,107,107,0.14)' };
  }
  return { color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' };
}

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

function formatStageLabel(value: string | null | undefined) {
  if (!value) return 'n/a';
  return value.replace(/_/g, ' ');
}

function formatRunTriggerLabel(value: string | null | undefined) {
  switch ((value ?? '').toLowerCase()) {
    case 'auto':
      return 'Auto-started';
    case 'manual':
      return 'Manual';
    case 'resume':
      return 'Resumed';
    default:
      return value ?? 'n/a';
  }
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

function buildRunChangeSummary(
  report: InvestigationResponse | null,
  runDetail: InvestigationRunDetailResponse | null
) {
  if (!report?.active_run) {
    return {
      headline: 'No active run is attached yet.',
      detail: 'Open an investigation to create or reopen a persistent run.',
    };
  }

  const run = report.active_run;
  if (run.trigger_type === 'auto' && run.status === 'queued') {
    return {
      headline: 'The system auto-started this run after launch activity crossed the threshold.',
      detail: run.summary ?? 'This queued run is waiting in auto triage before deeper follow-through happens.',
    };
  }

  if (runDetail && runDetail.timeline.length > 0) {
    const latest = runDetail.timeline[runDetail.timeline.length - 1]!;
    return {
      headline: latest.label,
      detail: latest.detail,
    };
  }

  return {
    headline: `Run is currently ${run.status}.`,
    detail: run.summary ?? `This workspace is attached to the ${formatStageLabel(run.current_stage)} stage.`,
  };
}

function compactNumber(value: number | null | undefined) {
  if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
  return Intl.NumberFormat('en-US', { notation: 'compact', maximumFractionDigits: 2 }).format(value);
}

function formatPercent(value: number | null | undefined, digits = 2) {
  if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
  return `${value.toFixed(digits)}%`;
}

function clampPercent(value: number) {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(100, value));
}

function participantWalletCount(report: InvestigationResponse) {
  return (
    report.internal.token.participant_wallet_count ??
    report.internal.wallet_structure.participant_wallet_count ??
    report.internal.wallet_structure.holder_count
  );
}

function indexedHolderCount(report: InvestigationResponse) {
  return report.contract_intelligence.indexed_holder_count ?? report.contract_intelligence.holder_count;
}

function gradeMeter(value: string | null | undefined) {
  switch ((value ?? '').trim().toUpperCase()) {
    case 'A':
      return 92;
    case 'B':
      return 74;
    case 'C':
      return 58;
    case 'D':
      return 34;
    case 'F':
      return 12;
    default:
      return 40;
  }
}

function toneStyles(tone: 'safe' | 'primary' | 'warn' | 'danger') {
  if (tone === 'safe') {
    return {
      color: 'var(--secondary-container)',
      background: 'rgba(0,255,163,0.08)',
      border: 'rgba(0,255,163,0.16)',
    };
  }
  if (tone === 'danger') {
    return {
      color: 'var(--danger)',
      background: 'rgba(255,107,107,0.1)',
      border: 'rgba(255,107,107,0.18)',
    };
  }
  if (tone === 'warn') {
    return {
      color: 'var(--warning)',
      background: 'rgba(255,186,73,0.12)',
      border: 'rgba(255,186,73,0.18)',
    };
  }
  return {
    color: 'var(--primary)',
    background: 'rgba(105,137,255,0.1)',
    border: 'rgba(105,137,255,0.18)',
  };
}

function holderBadge(holder: InvestigationResponse['contract_intelligence']['top_holders'][number]) {
  if (holder.is_owner) return 'Owner';
  if (holder.address_type?.trim()) return holder.address_type;
  return 'Holder';
}

function deepResearchEntitlementKey(address: string) {
  return `mia:deep-research-entitlement:${address.toLowerCase()}`;
}

function sentence(value: string | null | undefined, fallback: string) {
  const text = value?.trim();
  if (!text) return fallback;
  return text.endsWith('.') ? text : `${text}.`;
}

function plainVerdictLabel(value: string | null | undefined) {
  const verdict = (value ?? '').toLowerCase();
  if (verdict.includes('avoid')) return 'Avoid for now';
  if (verdict.includes('high conviction')) return 'Strong but still needs discipline';
  if (verdict.includes('watch')) return 'Watch, not chase';
  if (verdict.includes('speculative')) return 'Speculative setup';
  return value ?? 'Watch';
}

function decisionScorecard(report: InvestigationResponse): ResolvedDecisionScorecard {
  if (report.internal.agent_scorecard) {
    return {
      decision_score: report.internal.agent_scorecard.score,
      verdict: report.internal.agent_scorecard.label,
      confidence_label: report.internal.agent_scorecard.confidence_label,
      primary_reason: report.internal.agent_scorecard.primary_reason,
      primary_risk: report.internal.agent_scorecard.primary_risk,
      subscores: [],
    };
  }

  return {
    decision_score: report.analysis.score ?? null,
    verdict: report.analysis.label || report.analysis.verdict || 'Monitoring',
    confidence_label: report.analysis.confidence,
    primary_reason:
      report.analysis.primary_reason || report.analysis.executive_summary,
    primary_risk:
      report.analysis.primary_risk ??
      report.analysis.risks[0] ??
      'The setup is active, but the dominant risk is not isolated yet.',
    subscores: [],
  };
}

function decisionVerdict(report: InvestigationResponse) {
  return decisionScorecard(report).verdict || report.analysis.verdict;
}

function decisionConfidence(report: InvestigationResponse) {
  return decisionScorecard(report).confidence_label || report.analysis.confidence;
}

function decisionSubscore(
  report: InvestigationResponse,
  id: string
): DecisionSubscoreResponse | undefined {
  return decisionScorecard(report).subscores.find((item) => item.id === id);
}

function hasAgentScore(report: InvestigationResponse) {
  return decisionScorecard(report).decision_score !== null;
}

function displayInvestigationScore(report: InvestigationResponse) {
  const score = decisionScorecard(report).decision_score;
  return score !== null ? `${score}/100` : 'No score yet';
}

function deepResearchBadge(report: InvestigationResponse): string | null {
  if (report.deep_research.report_cached) return 'Deep researched';
  if (report.deep_research.auto_requested) return 'Deep research queued';
  return null;
}

function scoreStatusLine(report: InvestigationResponse) {
  if (hasAgentScore(report)) {
    if (report.deep_research.score_enriched) {
      return `MIA Agent Score ${decisionScorecard(report).decision_score} · enriched by deep research`;
    }
    return `MIA Agent Score ${decisionScorecard(report).decision_score} · ${decisionConfidence(report)} confidence`;
  }

  return `AI score locked until activity clears ${report.deep_research.ai_score_gate_tx_count} total transactions or a deep research report exists · ${decisionConfidence(report)} confidence`;
}

function subscoreTone(score: number): 'safe' | 'primary' | 'warn' | 'danger' {
  if (score >= 72) return 'safe';
  if (score >= 56) return 'primary';
  if (score >= 40) return 'warn';
  return 'danger';
}

function buildDecisionSummary(report: InvestigationResponse, executionPlan: MiaExecutionPlan) {
  const scorecard = decisionScorecard(report);
  return `${plainVerdictLabel(scorecard.verdict)}. ${sentence(scorecard.primary_reason, executionPlan.stance)}`;
}

function buildWhyItMattersSummary(report: InvestigationResponse) {
  const pieces: string[] = [];
  const scorecard = decisionScorecard(report);

  if (scorecard.decision_score !== null) {
    pieces.push(`MIA's unified investigation score is ${scorecard.decision_score}/100`);
  } else {
    pieces.push('MIA is still collecting enough activity to justify an AI score');
  }
  pieces.push(scorecard.primary_reason.replace(/\.$/, '').toLowerCase());
  if (report.internal.operator_family.confidence !== 'low') {
    pieces.push(
      `operator-pattern risk is ${report.internal.operator_family.confidence} confidence across ${report.internal.operator_family.related_launch_count} related launches`
    );
  }

  if (pieces.length === 0) {
    return 'MIA found enough activity to keep the token on the board, but there is no single dominant catalyst yet.';
  }

  return `${pieces.slice(0, 3).join(', ')}, which is why the token is being treated as active right now.`;
}

function buildRiskSummary(report: InvestigationResponse) {
  return decisionScorecard(report).primary_risk;
}

function scoreSourceLabel(report: InvestigationResponse) {
  if (report.deep_research.score_enriched) return 'AI score enriched by deep research';
  if (hasAgentScore(report)) return 'AI score from live investigation evidence';
  return 'AI score not unlocked yet';
}

function deepResearchSummary(report: InvestigationResponse) {
  if (report.deep_research.report_cached) {
    return 'A saved deep research report exists in the database and is now part of this read.';
  }
  if (report.deep_research.auto_requested) {
    return 'Deep research has been queued automatically and will enrich this token once the report finishes.';
  }
  return 'No deep research report is attached yet. You can still open it manually when the token deserves a deeper source pass.';
}

function scoreReasonSummary(report: InvestigationResponse) {
  if (hasAgentScore(report)) {
    return sentence(
      decisionScorecard(report).primary_reason,
      'The AI score is live, but the primary reason is still being resolved.'
    );
  }

  return `MIA keeps scoring locked until the token clears ${report.deep_research.ai_score_gate_tx_count} total transactions or a deep research report is available.`;
}

function InvestigationScorePanel({ report }: { report: InvestigationResponse }) {
  const scorecard = decisionScorecard(report);

  return (
    <section
      className="rounded-[24px] border p-5"
      style={{ background: 'rgba(23,31,49,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}
    >
      <div
        style={{
          fontSize: 10,
          fontWeight: 800,
          color: '#94a0c2',
          letterSpacing: '0.16em',
          textTransform: 'uppercase',
          fontFamily: 'Manrope, sans-serif',
          marginBottom: 14,
        }}
      >
        AI Score + Deep Research
      </div>
      <div className="grid gap-3 md:grid-cols-3">
        <div
          className="rounded-2xl border p-4"
          style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(255,255,255,0.08)' }}
        >
          <div className="text-[10px] font-extrabold uppercase tracking-[0.12em]" style={{ color: '#6f8dff', fontFamily: 'Manrope, sans-serif' }}>
            Score Source
          </div>
          <div className="mt-2 text-sm font-bold" style={{ color: '#edf1ff', fontFamily: 'Manrope, sans-serif' }}>
            {scoreSourceLabel(report)}
          </div>
          <p className="mt-2 text-sm leading-7" style={{ color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif' }}>
            {scoreReasonSummary(report)}
          </p>
        </div>

        <div
          className="rounded-2xl border p-4"
          style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(255,255,255,0.08)' }}
        >
          <div className="text-[10px] font-extrabold uppercase tracking-[0.12em]" style={{ color: '#36efb6', fontFamily: 'Manrope, sans-serif' }}>
            Current Result
          </div>
          <div className="mt-2 text-sm font-bold" style={{ color: '#edf1ff', fontFamily: 'Manrope, sans-serif' }}>
            {hasAgentScore(report) ? `${scorecard.decision_score}/100 · ${plainVerdictLabel(scorecard.verdict)}` : 'No score yet'}
          </div>
          <p className="mt-2 text-sm leading-7" style={{ color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif' }}>
            {sentence(scorecard.primary_risk, 'The dominant risk has not been isolated yet.')}
          </p>
        </div>

        <div
          className="rounded-2xl border p-4"
          style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(255,255,255,0.08)' }}
        >
          <div className="text-[10px] font-extrabold uppercase tracking-[0.12em]" style={{ color: '#ffd166', fontFamily: 'Manrope, sans-serif' }}>
            Deep Research State
          </div>
          <div className="mt-2 text-sm font-bold" style={{ color: '#edf1ff', fontFamily: 'Manrope, sans-serif' }}>
            {deepResearchBadge(report) ?? 'Not attached'}
          </div>
          <p className="mt-2 text-sm leading-7" style={{ color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif' }}>
            {deepResearchSummary(report)}
          </p>
        </div>
      </div>
    </section>
  );
}

function buildMlSummary(report: InvestigationResponse, proofHeadline: string) {
  if (report.internal.alpha_context) {
    return `MIA's proof support layer currently carries a live rank snapshot at #${report.internal.alpha_context.rank} with support score ${report.internal.alpha_context.alpha_score.toFixed(1)}. ${proofHeadline}`;
  }

  return `MIA still attaches its proof support and replay system here, even when the token does not yet carry a live support rank. ${proofHeadline}`;
}

function buildVisualSignals(report: InvestigationResponse): {
  label: string;
  value: string;
  note: string;
  meter: number;
  tone: 'safe' | 'primary' | 'warn' | 'danger';
  breakdown: { label: string; value: string }[];
}[] {
  const scorecard = decisionScorecard(report);
  const market = decisionSubscore(report, 'market_structure');
  const wallet = decisionSubscore(report, 'wallet_structure');
  const builder = decisionSubscore(report, 'builder_history');
  const operator = decisionSubscore(report, 'operator_family');
  const ml = decisionSubscore(report, 'ml_alignment');

  return [
    {
      label: 'Investigation Score',
      value: scorecard.decision_score !== null ? `${scorecard.decision_score}/100` : 'No score yet',
      note:
        scorecard.decision_score !== null
          ? scorecard.primary_reason
          : 'AI scoring is held back until this token clears the minimum activity gate.',
      meter: clampPercent(scorecard.decision_score ?? 0),
      breakdown: [
        { label: 'Current read', value: plainVerdictLabel(scorecard.verdict) },
        { label: 'Confidence', value: scorecard.confidence_label },
      ],
      tone: subscoreTone(scorecard.decision_score ?? 32),
    },
    {
      label: 'Market Structure',
      value: market ? `${market.score}/100` : 'n/a',
      note: market?.summary ?? `${report.internal.token.volume_bnb.toFixed(2)} BNB flow tracked`,
      meter: clampPercent(market?.score ?? 50),
      breakdown: [
        { label: 'Buys / sells', value: `${report.internal.token.buy_count} / ${report.internal.token.sell_count}` },
        { label: 'Volume', value: `${report.internal.token.volume_bnb.toFixed(2)} BNB` },
      ],
      tone: subscoreTone(market?.score ?? 50),
    },
    {
      label: 'Wallet Structure',
      value: wallet ? `${wallet.score}/100` : 'n/a',
      note: wallet?.summary ?? report.internal.wallet_structure.summary,
      meter: clampPercent(wallet?.score ?? 50),
      breakdown: [
        { label: 'Participant wallets', value: String(participantWalletCount(report)) },
        { label: 'Indexed holders', value: compactNumber(indexedHolderCount(report)) },
      ],
      tone: subscoreTone(wallet?.score ?? 50),
    },
    {
      label: 'Operator Pattern',
      value: operator ? `${operator.score}/100` : 'n/a',
      note: report.internal.operator_family.summary,
      meter: clampPercent(operator?.score ?? 50),
      breakdown: [
        { label: 'Confidence', value: report.internal.operator_family.confidence },
        { label: 'Related launches', value: String(report.internal.operator_family.related_launch_count) },
      ],
      tone: subscoreTone(operator?.score ?? 50),
    },
    {
      label: 'Builder + Proof',
      value: builder ? `${builder.score}/100` : 'n/a',
      note: builder?.summary ?? report.internal.deployer_memory?.summary ?? 'Builder memory is still thin on this token',
      meter: clampPercent(builder?.score ?? 50),
      breakdown: [
        { label: 'Builder', value: report.internal.deployer_memory?.trust_grade ?? 'n/a' },
        { label: 'Proof', value: ml ? `${ml.score}/100` : report.internal.alpha_context ? report.internal.alpha_context.alpha_score.toFixed(1) : 'n/a' },
      ],
      tone: subscoreTone(Math.round(((builder?.score ?? 50) + (ml?.score ?? 50)) / 2)),
    },
  ];
}

function buildLayerCards(report: InvestigationResponse, mlSummary: string): MiaLayerCard[] {
  const market = decisionSubscore(report, 'market_structure');
  const wallet = decisionSubscore(report, 'wallet_structure');
  const builder = decisionSubscore(report, 'builder_history');
  const operator = decisionSubscore(report, 'operator_family');
  const ml = decisionSubscore(report, 'ml_alignment');

  return [
    {
      id: 'market',
      title: 'Market Layer',
      kicker: 'Flow and trade structure',
      summary: market?.summary ?? 'Flow is still being resolved.',
      humanText: `MIA sees ${report.internal.token.buy_count} buys versus ${report.internal.token.sell_count} sells with ${report.internal.token.volume_bnb.toFixed(2)} BNB in tracked volume. This tells you whether the setup is being supported or fading in real time.`,
      analystPoints: [
        `${report.internal.token.buy_count} buys / ${report.internal.token.sell_count} sells`,
        `${report.internal.token.volume_bnb.toFixed(2)} BNB tracked volume`,
        market?.summary ?? 'Market structure summary unavailable',
      ],
      meter: clampPercent(market?.score ?? 50),
      tone: subscoreTone(market?.score ?? 50),
    },
    {
      id: 'wallet',
      title: 'Wallet Layer',
      kicker: 'Participation and concentration',
      summary: wallet?.summary ?? report.internal.wallet_structure.summary,
      humanText: `MIA checks whether participation is broad enough to trust the move. It tracks participant wallets, cluster risk, repeated early wallets, and whether too few wallets still control too much of the flow.`,
      analystPoints: [
        `${compactNumber(participantWalletCount(report))} participant wallets`,
        `${compactNumber(indexedHolderCount(report))} indexed holders`,
        `${report.internal.wallet_structure.probable_cluster_wallets} probable cluster wallet(s)`,
      ],
      meter: clampPercent(wallet?.score ?? 50),
      tone: subscoreTone(wallet?.score ?? 50),
    },
    {
      id: 'operator',
      title: 'Operator Pattern',
      kicker: 'Cross-launch sybil and migration pattern risk',
      summary: operator?.summary ?? report.internal.operator_family.summary,
      humanText: `This layer looks for likely coordinated operator-family behavior: repeated wallets across launches, seller wallets that later reappear early in new launches, and launch overlap across different deployer wallets. It is pattern warning, not an identity claim.`,
      analystPoints: [
        `${report.internal.operator_family.confidence} confidence`,
        `${report.internal.operator_family.related_launch_count} related launches`,
        `${report.internal.operator_family.repeated_wallet_count} repeated wallets`,
      ],
      meter: clampPercent(operator?.score ?? 50),
      tone: subscoreTone(operator?.score ?? 50),
    },
    {
      id: 'builder',
      title: 'Builder Layer',
      kicker: 'Launch history and repeat behavior',
      summary: builder?.summary ?? report.internal.deployer_memory?.summary ?? 'Builder memory is thin, so this layer is weaker than usual.',
      humanText: report.internal.deployer_memory
        ? `MIA tracks whether this deployer repeatedly launches tokens and whether those launches graduate or fail. That is how the report decides whether the builder deserves trust or extra skepticism.`
        : 'No deployer memory has been attached yet, so this token is being judged more from current flow than historical builder behavior.',
      analystPoints: report.internal.deployer_memory
        ? [
            `${report.internal.deployer_memory.total_launches} launches tracked`,
            `${report.internal.deployer_memory.graduated_count} graduates`,
            `${report.internal.deployer_memory.rug_count} rugs`,
          ]
        : ['No deployer profile attached'],
      meter: clampPercent(builder?.score ?? gradeMeter(report.internal.deployer_memory?.trust_grade)),
      tone: subscoreTone(builder?.score ?? 50),
    },
    {
      id: 'ml',
      title: 'Proof Support Layer',
      kicker: 'Support ranking and replay evidence',
      summary:
        ml?.summary ??
        (report.internal.alpha_context
          ? `MIA's proof support layer currently places this token at #${report.internal.alpha_context.rank}.`
          : 'The proof support stack is attached, but this token does not yet carry a live support rank.'),
      humanText: mlSummary,
      analystPoints: report.internal.alpha_context
        ? [
            `Rank #${report.internal.alpha_context.rank}`,
            `Support score ${report.internal.alpha_context.alpha_score.toFixed(1)}`,
            report.internal.alpha_context.rationale,
          ]
        : ['No live support rank', 'Replay layer still attached for proof'],
      meter: clampPercent(ml?.score ?? (report.internal.alpha_context ? report.internal.alpha_context.alpha_score : 42)),
      tone: subscoreTone(ml?.score ?? 42),
    },
  ];
}

export default function MiaInvestigationPage() {
  return (
    <Suspense
      fallback={
        <>
          <ObsidianNav />
          <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
            <section
              className="rounded-xl border p-6 text-sm"
              style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(141,144,161,0.15)', color: 'var(--on-surface-variant)' }}
            >
              Loading investigation workspace...
            </section>
          </main>
        </>
      }
    >
      <MiaInvestigationClient />
    </Suspense>
  );
}

function MiaInvestigationClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const initialQuery = searchParams.get('q') ?? '';
  const [input, setInput] = useState(initialQuery);
  const [report, setReport] = useState<InvestigationResponse | null>(null);
  const [runDetail, setRunDetail] = useState<InvestigationRunDetailResponse | null>(null);
  const [activeRun, setActiveRun] = useState<InvestigationRunSummary | null>(null);
  const [resolvedToken, setResolvedToken] = useState<TokenSummary | null>(null);
  const [outcomes, setOutcomes] = useState<MiaOutcomeCard[]>([]);
  const [proofHeadline, setProofHeadline] = useState('Measured edge will appear here once proof loads.');
  const [proofSupport, setProofSupport] = useState('Backtesting and shadow-model evaluation are attached to every serious setup.');
  const [proofBadge, setProofBadge] = useState('Proof layer');
  const [deepResearchPreview, setDeepResearchPreview] = useState<DeepResearchPreviewResponse | null>(null);
  const [deepResearchStatus, setDeepResearchStatus] = useState<DeepResearchStatusResponse | null>(null);
  const [deepResearchReport, setDeepResearchReport] = useState<DeepResearchReportResponse | null>(null);
  const [deepResearchEntitlement, setDeepResearchEntitlement] = useState<string | null>(null);
  const [deepResearchLoading, setDeepResearchLoading] = useState(false);
  const [deepResearchError, setDeepResearchError] = useState<string | null>(null);
  const [runDetailError, setRunDetailError] = useState<string | null>(null);
  const [runActionLoading, setRunActionLoading] = useState<'watching' | 'escalated' | 'archived' | null>(null);
  const [runActionError, setRunActionError] = useState<string | null>(null);
  const [watchSaveLoading, setWatchSaveLoading] = useState<'token' | 'builder' | null>(null);
  const [watchSaveNotice, setWatchSaveNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [briefCopied, setBriefCopied] = useState(false);
  const [viewMode, setViewMode] = useState<MiaViewMode>('quick');
  const [readMode, setReadMode] = useState<MiaReadMode>('human');
  const [activeSection, setActiveSection] = useState<MiaWorkspaceSection>('overview');
  const [showDetail, setShowDetail] = useState(false);
  const [openLayers, setOpenLayers] = useState<Record<string, boolean>>({
    market: true,
    wallet: false,
    operator: false,
    builder: false,
    ml: false,
  });

  const runInvestigation = useCallback(
    async (rawQuery: string) => {
      const query = rawQuery.trim();
      if (!query) return;

      setLoading(true);
      setError(null);
      setOutcomes([]);
      setRunDetail(null);
      setActiveRun(null);
      setRunActionError(null);
      setWatchSaveNotice(null);
      setDeepResearchPreview(null);
      setDeepResearchStatus(null);
      setDeepResearchReport(null);
      setDeepResearchEntitlement(null);
      setDeepResearchError(null);
      try {
        let address = query;
        let resolved: TokenSummary | null = null;

        if (!looksLikeAddress(query)) {
          const tokenList = await api.tokens.list({ q: query, limit: 8, sort: 'volume' });
          resolved =
            tokenList.data.find((token) => token.symbol?.toLowerCase() === query.toLowerCase()) ??
            tokenList.data.find((token) => token.name?.toLowerCase() === query.toLowerCase()) ??
            tokenList.data[0] ??
            null;

          if (!resolved) {
            throw new Error(`No token matched "${query}" in the indexed Four.Meme feed.`);
          }

          address = resolved.contract_address;
        }

        const [investigation, backtest, mlEval] = await Promise.all([
          api.tokens.investigation(address),
          api.alpha.backtest(48, 180).catch(() => null),
          api.ml.alphaEval(168).catch(() => null),
        ]);

        const proof = buildMiaProofSnapshot(backtest, mlEval);
        const tokenFromReport: TokenSummary = {
          contract_address: investigation.internal.token.contract_address,
          name: investigation.internal.token.name,
          symbol: investigation.internal.token.symbol,
          deployer_address: investigation.internal.token.deployer_address,
          deployed_at: investigation.internal.token.deployed_at,
          block_number: investigation.internal.token.block_number,
          buy_count: investigation.internal.token.buy_count,
          sell_count: investigation.internal.token.sell_count,
          total_tx: investigation.internal.token.buy_count + investigation.internal.token.sell_count,
          volume_bnb: investigation.internal.token.volume_bnb,
          composite_score: investigation.internal.risk?.composite_score ?? null,
          risk_category: (investigation.internal.risk?.risk_category as 'low' | 'medium' | 'high' | null) ?? null,
          ai_scored: investigation.deep_research.ai_score_enabled,
          deep_researched: investigation.deep_research.report_cached,
        };

        setResolvedToken(resolved ?? tokenFromReport);
        setReport(investigation);
        setActiveRun(investigation.active_run);
        setProofHeadline(proof.headline);
        setProofSupport(proof.support);
        setProofBadge(proof.badge);
        setOutcomes(backtest ? buildRecentOutcomeCards(backtest.rows, 3) : []);
        router.replace(`/mia?q=${encodeURIComponent(query)}`);
      } catch (err) {
        setReport(null);
        setResolvedToken(null);
        setOutcomes([]);
        setError(err instanceof Error ? err.message : 'Investigation failed');
      } finally {
        setLoading(false);
      }
    },
    [router]
  );

  useEffect(() => {
    if (!initialQuery) return;
    runInvestigation(initialQuery);
  }, [initialQuery, runInvestigation]);

  useEffect(() => {
    const address = report?.internal.token.contract_address;
    if (!address) return;

    const loadDeepResearch = async () => {
      setDeepResearchLoading(true);
      setDeepResearchError(null);

      try {
        const [preview, status] = await Promise.all([
          api.tokens.deepResearchPreview(address),
          api.tokens.deepResearchStatus(address),
        ]);

        setDeepResearchPreview(preview);
        setDeepResearchStatus(status);

        const savedEntitlement = window.localStorage.getItem(deepResearchEntitlementKey(address));
        setDeepResearchEntitlement(savedEntitlement);
        if (!savedEntitlement) {
          setDeepResearchReport(null);
          return;
        }

        const premium = await api.tokens.deepResearchReport(address, savedEntitlement);
        setDeepResearchReport(premium.data);

        const entitlementHeader = premium.headers.get('x-mia-entitlement');
        if (entitlementHeader) {
          window.localStorage.setItem(deepResearchEntitlementKey(address), entitlementHeader);
          setDeepResearchEntitlement(entitlementHeader);
        } else if (premium.data.entitlement?.access_token) {
          setDeepResearchEntitlement(premium.data.entitlement.access_token);
          window.localStorage.setItem(
            deepResearchEntitlementKey(address),
            premium.data.entitlement.access_token
          );
        }
      } catch (err) {
        setDeepResearchReport(null);
        setDeepResearchEntitlement(null);
        setDeepResearchError(err instanceof Error ? err.message : 'Deep Research is unavailable right now.');
      } finally {
        setDeepResearchLoading(false);
      }
    };

    void loadDeepResearch();
  }, [report?.internal.token.contract_address]);

  const summaryTone = useMemo(() => statusTone(report ? decisionVerdict(report) : 'watch'), [report]);
  const executionPlan = useMemo<MiaExecutionPlan | null>(() => (report ? buildMiaExecutionPlan(report) : null), [report]);
  const actionBrief = useMemo(
    () => (report && executionPlan ? buildMiaActionBrief(report, executionPlan) : null),
    [report, executionPlan]
  );
  const decisionSummary = useMemo(
    () => (report && executionPlan ? buildDecisionSummary(report, executionPlan) : null),
    [report, executionPlan]
  );
  const whyItMattersSummary = useMemo(
    () => (report ? buildWhyItMattersSummary(report) : null),
    [report]
  );
  const riskSummary = useMemo(() => (report ? buildRiskSummary(report) : null), [report]);
  const mlSummary = useMemo(
    () => (report ? buildMlSummary(report, proofHeadline) : null),
    [report, proofHeadline]
  );
  const visualSignals = useMemo(() => (report ? buildVisualSignals(report) : []), [report]);
  const layerCards = useMemo(
    () => (report && mlSummary ? buildLayerCards(report, mlSummary) : []),
    [report, mlSummary]
  );
  const visibleSignals = useMemo(
    () => (viewMode === 'quick' ? visualSignals.slice(0, 4) : visualSignals),
    [viewMode, visualSignals]
  );
  const visibleLayerCards = useMemo(
    () => (viewMode === 'quick' ? layerCards.slice(0, 3) : layerCards),
    [viewMode, layerCards]
  );
  const workspaceSections = useMemo<
    {
      value: MiaWorkspaceSection;
      label: string;
      description: string;
      icon: React.ReactNode;
    }[]
  >(
    () => [
      {
        value: 'overview',
        label: 'Overview',
        description: 'Start with the short evidence read and the strongest signals.',
        icon: <FaBrain size={15} />,
      },
      {
        value: 'layers',
        label: 'Evidence Layers',
        description: 'Open the wallet, builder, narrative, and ML evidence one layer at a time.',
        icon: <FaChartLine size={15} />,
      },
      {
        value: 'timeline',
        label: 'Timeline',
        description: 'See how this run was created, updated, and what changed most recently.',
        icon: <FaDatabase size={15} />,
      },
      {
        value: 'proof',
        label: 'Proof',
        description: 'See replay support, ML context, and what has real validation attached.',
        icon: <FaWaveSquare size={15} />,
      },
      {
        value: 'sources',
        label: 'Sources',
        description: 'Inspect citations, fallback status, and raw source provenance.',
        icon: <FaLink size={15} />,
      },
      {
        value: 'tools',
        label: 'Tools',
        description: 'Open Ask MIA, monitoring setup, and deeper research tools only when needed.',
        icon: <FaBolt size={15} />,
      },
    ],
    []
  );

  useEffect(() => {
    if (!actionBrief) return;
    localStorage.setItem('mia:latest-action-brief', JSON.stringify(actionBrief));
    setBriefCopied(false);
  }, [actionBrief]);

  useEffect(() => {
    if (!report) return;
    setOpenLayers({
      market: true,
      wallet: false,
      operator: false,
      builder: false,
      ml: false,
    });
    setActiveSection('overview');
    setShowDetail(false);
  }, [report]);

  useEffect(() => {
    setActiveRun(report?.active_run ?? null);
    setRunActionError(null);
  }, [report]);

  useEffect(() => {
    const runId = activeRun?.run_id;
    if (!runId) {
      setRunDetail(null);
      setRunDetailError(null);
      return;
    }

    let active = true;
    setRunDetailError(null);

    api.investigations
      .getRunDetail(runId)
      .then((detail) => {
        if (!active) return;
        setRunDetail(detail);
      })
      .catch((err) => {
        if (!active) return;
        setRunDetail(null);
        setRunDetailError(err instanceof Error ? err.message : 'Failed to load run detail');
      });

    return () => {
      active = false;
    };
  }, [activeRun?.run_id]);

  const copyActionBrief = async () => {
    if (!actionBrief) return;
    try {
      await navigator.clipboard.writeText(actionBrief.clipboardText);
      setBriefCopied(true);
    } catch {
      setBriefCopied(false);
    }
  };

  const toggleLayer = (id: string) => {
    setOpenLayers((current) => ({
      ...current,
      [id]: !current[id],
    }));
  };

  const applyRunStatus = async (status: 'watching' | 'escalated' | 'archived') => {
    if (!activeRun) return;
    setRunActionLoading(status);
    setRunActionError(null);
    const payload = {
      status,
      reason:
        status === 'watching'
          ? `Monitoring reason: ${whyItMattersSummary ?? decisionSummary}`
          : status === 'escalated'
            ? `Escalation reason: ${riskSummary ?? decisionSummary}`
            : 'Archive reason: investigation was reviewed and moved out of the active workspace.',
      evidence_delta:
        status === 'archived'
          ? `Latest evidence delta before archive: ${runChangeSummary.detail}`
          : `Latest evidence delta: ${runChangeSummary.detail}`,
    };
    try {
      const updated = await api.investigations.updateRunStatus(activeRun.run_id, payload);
      setActiveRun(updated);
      const detail = await api.investigations.getRunDetail(updated.run_id);
      setRunDetail(detail);
      setRunDetailError(null);
    } catch (err) {
      setRunActionError(err instanceof Error ? err.message : 'Failed to update run status');
    } finally {
      setRunActionLoading(null);
    }
  };

  const saveWatchlistItem = async (entityKind: 'token' | 'builder') => {
    if (!report) return;
    const entityKey =
      entityKind === 'token'
        ? report.internal.token.contract_address
        : report.internal.token.deployer_address;
    const label =
      entityKind === 'token'
        ? `${report.internal.token.symbol ?? 'TOKEN'} watch`
        : `Builder ${shortAddress(report.internal.token.deployer_address, 8, 6)}`;

    setWatchSaveLoading(entityKind);
    setWatchSaveNotice(null);
    try {
      await api.investigations.createWatchlistItem({
        entity_kind: entityKind,
        entity_key: entityKey,
        label,
        source_run_id: currentRun?.run_id ?? null,
      });
      setWatchSaveNotice(
        entityKind === 'token'
          ? 'Token watch saved to the persistent watchlist.'
          : 'Builder watch saved to the persistent watchlist.'
      );
    } catch (err) {
      setWatchSaveNotice(err instanceof Error ? err.message : 'Failed to save watchlist item');
    } finally {
      setWatchSaveLoading(null);
    }
  };

  const currentRun = activeRun ?? report?.active_run ?? null;
  const runChangeSummary = useMemo(
    () => buildRunChangeSummary(report ? { ...report, active_run: currentRun } : null, runDetail),
    [report, currentRun, runDetail]
  );

  return (
    <>
      <ObsidianNav />
      <main
        className="obsidian-page min-h-screen px-3 pb-16 pt-4"
        style={{
          background:
            'radial-gradient(circle at top, rgba(111,141,255,0.14), transparent 32%), radial-gradient(circle at 20% 20%, rgba(54,239,182,0.08), transparent 28%), #08101d',
        }}
      >
        <div className="mx-auto w-full max-w-[430px] lg:hidden">
          <div
            className="mb-4 flex items-center gap-3 rounded-[14px] border px-4 py-3"
            style={{ background: 'rgba(18,24,38,0.9)', borderColor: 'rgba(111,141,255,0.2)' }}
          >
            <FaMagnifyingGlass size={16} color="#6f8dff" />
            <input
              value={input}
              onChange={(event) => setInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') runInvestigation(input);
              }}
              className="w-full border-none bg-transparent text-sm focus:ring-0"
              style={{ color: '#edf1ff', fontFamily: 'Space Grotesk, sans-serif' }}
              placeholder="Enter ticker or 0x address..."
            />
            {input.trim() ? (
              <button
                onClick={() => runInvestigation(input)}
                disabled={loading}
                className="rounded-lg px-3 py-2 text-[11px] font-extrabold uppercase tracking-[0.1em] disabled:opacity-50"
                style={{ background: '#6f8dff', color: '#081736', fontFamily: 'Manrope, sans-serif' }}
              >
                {loading ? '...' : 'Go'}
              </button>
            ) : null}
          </div>

          {loading && !report && (
            <div className="py-10 text-center">
              <div className="text-[11px] font-bold uppercase tracking-[0.18em]" style={{ color: '#6f8dff', fontFamily: 'Manrope, sans-serif' }}>
                MIA ANALYZING
              </div>
              <p className="mt-3 text-sm" style={{ color: '#94a0c2', fontFamily: 'Space Grotesk, sans-serif' }}>
                Scanning on-chain data, wallet structure, builder history...
              </p>
            </div>
          )}

          {error && (
            <section
              className="rounded-xl border p-4 text-sm"
              style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}
            >
              {error}
            </section>
          )}

          {!report && !loading && !error && (
            <div className="py-10 text-center">
              <div style={{ fontSize: 40, marginBottom: 12 }}>🔍</div>
              <h1 className="font-headline text-2xl font-extrabold tracking-tight" data-testid="mia-page-heading">
                Open a token investigation.
              </h1>
              <p className="mx-auto mt-3 max-w-[22rem] text-sm leading-7" style={{ color: '#94a0c2', fontFamily: 'Space Grotesk, sans-serif' }}>
                Enter a ticker like PEPE or a contract address to open the full MIA investigation screen.
              </p>
              <div className="mt-4 flex flex-wrap justify-center gap-2">
                <span className="rounded-full border px-3 py-1 text-[10px] font-bold uppercase tracking-[0.12em]" style={{ color: '#a8b8ff', borderColor: 'rgba(111,141,255,0.16)', background: 'rgba(111,141,255,0.08)' }}>
                  Flow
                </span>
                <span className="rounded-full border px-3 py-1 text-[10px] font-bold uppercase tracking-[0.12em]" style={{ color: '#36efb6', borderColor: 'rgba(54,239,182,0.16)', background: 'rgba(54,239,182,0.08)' }}>
                  Wallets
                </span>
                <span className="rounded-full border px-3 py-1 text-[10px] font-bold uppercase tracking-[0.12em]" style={{ color: '#ffd166', borderColor: 'rgba(255,209,102,0.16)', background: 'rgba(255,209,102,0.08)' }}>
                  Builder
                </span>
                <span className="rounded-full border px-3 py-1 text-[10px] font-bold uppercase tracking-[0.12em]" style={{ color: '#94a0c2', borderColor: 'rgba(148,160,194,0.16)', background: 'rgba(255,255,255,0.04)' }}>
                  Proof
                </span>
              </div>
            </div>
          )}
        {report && executionPlan && (
          <>
            <section style={{ animation: 'mia-fadeUp 0.4s ease' }}>
              <div style={{ padding: '0 0 12px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 2, flexWrap: 'wrap' }}>
                  <span style={{ fontSize: 18, fontWeight: 900, fontFamily: 'Manrope', color: '#edf1ff' }}>
                    {resolvedToken?.symbol ?? report.internal.token.symbol ?? 'TOKEN'}
                  </span>
                  <span
                    style={{
                      color:
                        report.internal.risk?.risk_category === 'high'
                          ? '#ff8080'
                          : report.internal.risk?.risk_category === 'low'
                            ? '#36efb6'
                            : '#ffd166',
                      background:
                        report.internal.risk?.risk_category === 'high'
                          ? 'rgba(255,128,128,0.12)'
                          : report.internal.risk?.risk_category === 'low'
                            ? 'rgba(54,239,182,0.1)'
                            : 'rgba(255,209,102,0.1)',
                      fontSize: 9,
                      fontWeight: 800,
                      letterSpacing: '0.14em',
                      padding: '2px 7px',
                      borderRadius: 4,
                      fontFamily: 'Manrope',
                      textTransform: 'uppercase',
                    }}
                  >
                    {(report.internal.risk?.risk_category ?? 'medium').toUpperCase()} risk
                  </span>
                  {deepResearchBadge(report) ? (
                    <span
                      style={{
                        color: '#6f8dff',
                        background: 'rgba(111,141,255,0.12)',
                        fontSize: 9,
                        fontWeight: 800,
                        letterSpacing: '0.14em',
                        padding: '2px 7px',
                        borderRadius: 4,
                        fontFamily: 'Manrope',
                        textTransform: 'uppercase',
                      }}
                    >
                      {deepResearchBadge(report)}
                    </span>
                  ) : null}
                </div>
                <span style={{ fontSize: 11, color: '#94a0c2', fontFamily: 'Roboto Mono' }}>
                  {report.internal.token.contract_address}
                </span>
              </div>

              <section
                data-testid="mia-mobile-hero-card"
                style={{
                  marginBottom: 14,
                  background: 'linear-gradient(135deg, rgba(111,141,255,0.14), rgba(12,16,24,0.98))',
                  borderRadius: 16,
                  border: '1px solid rgba(111,141,255,0.22)',
                  padding: '18px 16px',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
                  <ScoreOrb
                    score={decisionScorecard(report).decision_score ?? 0}
                    tone={subscoreTone(decisionScorecard(report).decision_score ?? 32)}
                    label="MIA"
                  />
                  <div style={{ flex: 1 }}>
                    <div style={{ marginBottom: 8 }}>
                      <span
                        style={{
                          background: summaryTone.background,
                          color: summaryTone.color,
                          border: `1px solid ${summaryTone.background}`,
                          borderRadius: 6,
                          padding: '7px 14px',
                          fontSize: 13,
                          fontWeight: 800,
                          letterSpacing: '0.12em',
                          textTransform: 'uppercase',
                          whiteSpace: 'nowrap',
                          fontFamily: 'Manrope, sans-serif',
                        }}
                      >
                        {plainVerdictLabel(decisionVerdict(report))}
                      </span>
                    </div>
                    <div style={{ fontSize: 12, color: '#adb6d0', fontFamily: 'Space Grotesk', lineHeight: 1.5 }}>
                      {scoreStatusLine(report)}
                    </div>
                  </div>
                </div>

                <div style={{ display: 'flex', gap: 8, marginTop: 14 }}>
                  <button
                    type="button"
                    data-testid="mia-intent-quick"
                    onClick={() => {
                      setViewMode('quick');
                      setReadMode('human');
                      setActiveSection('overview');
                    }}
                    style={{
                      flex: 1,
                      padding: '10px 0',
                      borderRadius: 10,
                      background: '#6f8dff',
                      border: 'none',
                      color: '#081736',
                      fontSize: 11,
                      fontWeight: 800,
                      cursor: 'pointer',
                      letterSpacing: '0.12em',
                      textTransform: 'uppercase',
                      fontFamily: 'Manrope',
                    }}
                  >
                    Quick Read
                  </button>
                  <button
                    type="button"
                    data-testid="mia-intent-deep"
                    onClick={() => {
                      setViewMode('full');
                      setReadMode('analyst');
                      setActiveSection('tools');
                      setShowDetail(true);
                    }}
                    style={{
                      flex: 1,
                      padding: '10px 0',
                      borderRadius: 10,
                      background: 'rgba(255,255,255,0.05)',
                      border: '1px solid rgba(255,255,255,0.1)',
                      color: '#adb6d0',
                      fontSize: 11,
                      fontWeight: 800,
                      cursor: 'pointer',
                      letterSpacing: '0.12em',
                      textTransform: 'uppercase',
                      fontFamily: 'Manrope',
                    }}
                  >
                    Deep Research
                  </button>
                </div>
              </section>

              <div style={{ marginBottom: 14 }}>
                <InvestigationScorePanel report={report} />
              </div>

              <section
                data-testid="mia-evidence-signals-card"
                style={{
                  marginBottom: 14,
                  background: 'rgba(23,31,49,0.9)',
                  borderRadius: 14,
                  border: '1px solid rgba(255,255,255,0.06)',
                  padding: '14px 16px',
                }}
              >
                <div style={{ fontSize: 10, fontWeight: 800, color: '#94a0c2', letterSpacing: '0.16em', textTransform: 'uppercase', fontFamily: 'Manrope', marginBottom: 12 }}>
                  Evidence Signals
                </div>
                {visibleSignals.map((signal) => (
                  <EvidenceSignalRow
                    key={signal.label}
                    label={signal.label}
                    value={signal.value}
                    note={signal.note}
                    meter={signal.meter}
                    tone={signal.tone}
                  />
                ))}
              </section>

              <section
                style={{
                  marginBottom: 14,
                  background: 'rgba(23,31,49,0.9)',
                  borderRadius: 14,
                  border: '1px solid rgba(255,255,255,0.06)',
                  padding: '14px 16px',
                }}
              >
                <div style={{ fontSize: 10, fontWeight: 800, color: '#6f8dff', letterSpacing: '0.16em', textTransform: 'uppercase', fontFamily: 'Manrope', marginBottom: 8 }}>
                  ✦ MIA Analysis
                </div>
                <p style={{ margin: 0, fontSize: 13, color: '#adb6d0', fontFamily: 'Space Grotesk', lineHeight: 1.65 }}>
                  {decisionSummary}
                </p>
              </section>

              <div style={{ padding: '0 0 14px' }}>
                <button
                  type="button"
                  onClick={() => setShowDetail((value) => !value)}
                  style={{
                    width: '100%',
                    padding: '11px',
                    borderRadius: 12,
                    background: 'rgba(255,255,255,0.04)',
                    border: '1px solid rgba(255,255,255,0.08)',
                    color: '#94a0c2',
                    fontSize: 11,
                    fontWeight: 700,
                    cursor: 'pointer',
                    fontFamily: 'Manrope',
                    letterSpacing: '0.12em',
                    textTransform: 'uppercase',
                  }}
                >
                  {showDetail ? 'Hide Details ↑' : 'Show More Details ↓'}
                </button>
              </div>

              {showDetail && (
                <>
                  <div style={{ marginBottom: 14 }}>
                    <Panel title="Detail Snapshot" icon={<FaDatabase size={14} />}>
                      <div className="grid grid-cols-2 gap-3">
                        <InfoPill label="Volume" value={`${report.internal.token.volume_bnb.toFixed(2)} BNB`} note="Tracked in the current read" />
                        <InfoPill label="Buy / Sell" value={`${report.internal.token.buy_count} / ${report.internal.token.sell_count}`} note="Current visible flow" />
                        <InfoPill label="Participants" value={compactNumber(participantWalletCount(report))} note={`${compactNumber(indexedHolderCount(report))} indexed holders`} />
                        <InfoPill label="Builder" value={report.internal.deployer_memory?.trust_grade ?? 'n/a'} note={report.internal.deployer_memory?.summary ?? 'Builder memory is still thin on this token.'} />
                        <InfoPill label="Proof" value={proofBadge} note={proofHeadline} />
                        <InfoPill label="Address" value={shortAddress(report.internal.token.contract_address, 10, 6)} note={report.internal.token.contract_address} />
                      </div>
                    </Panel>
                  </div>

                  {currentRun && (
                    <div style={{ marginBottom: 14 }}>
                      <Panel title="Run Console" icon={<FaWaveSquare size={14} />}>
                        <div className="grid gap-3">
                          <div className="flex flex-wrap gap-2">
                            <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.16em]" style={statusTone(currentRun.status)}>
                              {currentRun.status}
                            </span>
                            <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.16em]" style={{ color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }}>
                              {formatRunTriggerLabel(currentRun.trigger_type)}
                            </span>
                            <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.16em]" style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }}>
                              {formatStageLabel(currentRun.current_stage)}
                            </span>
                          </div>

                          <p className="text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                            {runChangeSummary.headline} {runChangeSummary.detail}
                          </p>

                          <div className="grid gap-2 sm:grid-cols-3">
                            <button type="button" data-testid="mia-run-action-watching" onClick={() => applyRunStatus('watching')} disabled={runActionLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(105,137,255,0.18)', color: 'var(--primary)', background: 'rgba(105,137,255,0.08)' }}>
                              {runActionLoading === 'watching' ? 'Updating...' : 'Mark Watching'}
                            </button>
                            <button type="button" data-testid="mia-run-action-escalated" onClick={() => applyRunStatus('escalated')} disabled={runActionLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(255,186,73,0.2)', color: 'var(--warning)', background: 'rgba(255,186,73,0.08)' }}>
                              {runActionLoading === 'escalated' ? 'Updating...' : 'Escalate Run'}
                            </button>
                            <button type="button" data-testid="mia-run-action-archived" onClick={() => applyRunStatus('archived')} disabled={runActionLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(141,144,161,0.18)', color: 'var(--outline)', background: 'rgba(255,255,255,0.05)' }}>
                              {runActionLoading === 'archived' ? 'Updating...' : 'Archive Run'}
                            </button>
                          </div>

                          <div className="grid gap-2 sm:grid-cols-2">
                            <button type="button" data-testid="mia-save-token-watch" onClick={() => saveWatchlistItem('token')} disabled={watchSaveLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(0,255,163,0.18)', color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }}>
                              {watchSaveLoading === 'token' ? 'Saving...' : 'Save Token Watch'}
                            </button>
                            <button type="button" data-testid="mia-save-builder-watch" onClick={() => saveWatchlistItem('builder')} disabled={watchSaveLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(141,144,161,0.18)', color: 'var(--primary)', background: 'rgba(255,255,255,0.05)' }}>
                              {watchSaveLoading === 'builder' ? 'Saving...' : 'Save Builder Watch'}
                            </button>
                          </div>

                          <div className="grid gap-2 sm:grid-cols-2">
                            <PrimaryActionTile testId="mia-open-runs-inbox" href={`/mia/runs?status=${encodeURIComponent(currentRun.status)}&trigger=${encodeURIComponent(currentRun.trigger_type)}`} label="Runs Inbox" caption="Compare runs" tone="primary" />
                            <PrimaryActionTile testId="mia-open-token-history" href={`/mia/token/${encodeURIComponent(currentRun.token_address)}`} label="Token History" caption="Open continuity" tone="safe" />
                          </div>

                          {runActionError ? (
                            <div className="rounded-xl border p-3 text-sm" style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}>
                              {runActionError}
                            </div>
                          ) : null}

                          {watchSaveNotice ? (
                            <div data-testid="mia-watch-save-notice" className="rounded-xl border p-3 text-sm" style={{ background: 'rgba(0,255,163,0.08)', borderColor: 'rgba(0,255,163,0.18)', color: 'var(--secondary-container)' }}>
                              {watchSaveNotice}
                            </div>
                          ) : null}
                        </div>
                      </Panel>
                    </div>
                  )}

                  <div style={{ marginBottom: 14 }}>
                    <Panel title="More Lanes" icon={<FaBolt size={14} />}>
                      <div className="grid gap-3">
                        <div className="-mx-1 overflow-x-auto px-1 pb-2">
                          <div className="flex min-w-max gap-3">
                            {workspaceSections.map((section) => (
                              <WorkspaceSectionButton
                                key={section.value}
                                section={section.value}
                                label={section.label}
                                description={section.description}
                                icon={section.icon}
                                active={activeSection === section.value}
                                onClick={() => setActiveSection(section.value)}
                              />
                            ))}
                          </div>
                        </div>

                        {currentRun ? (
                          <div data-testid="mia-run-console-summary" className="grid gap-3 md:grid-cols-2">
                            <RunConsoleChip label="Run type" value={formatRunTriggerLabel(currentRun.trigger_type)} note="Manual and auto runs share the same object." />
                            <RunConsoleChip label="What changed" value={runChangeSummary.headline} note={runChangeSummary.detail} />
                          </div>
                        ) : null}
                      </div>
                    </Panel>
                  </div>
                </>
              )}
            </section>

            {showDetail && activeSection === 'overview' && (
              <section className="grid gap-6 lg:grid-cols-[1.05fr_0.95fr]">
                <Panel title="What matters right now" icon={<FaBrain size={14} />}>
                  <div className="rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}>
                    <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                      Short evidence read
                    </p>
                    <p className="mt-2 text-base leading-7">{decisionSummary}</p>
                  </div>
                  <div className="mt-4 grid gap-3 sm:grid-cols-2">
                    <InfoPill
                      label="Operator pattern"
                      value={report.internal.operator_family.confidence}
                      note={report.internal.operator_family.summary}
                    />
                    <InfoPill
                      label="Builder memory"
                      value={report.internal.deployer_memory?.trust_grade ?? 'n/a'}
                      note={report.internal.deployer_memory?.summary ?? 'Builder history is still thin on this token.'}
                    />
                    <InfoPill
                      label="Proof support"
                      value={report.internal.alpha_context ? `Rank #${report.internal.alpha_context.rank}` : 'No live rank'}
                      note={mlSummary ?? 'Proof support is not attached yet.'}
                    />
                    <InfoPill
                      label="Proof status"
                      value={proofBadge}
                      note={proofSupport}
                    />
                  </div>
                </Panel>

                <Panel title="Coverage at a glance" icon={<FaChartLine size={14} />}>
                  <div className="grid gap-3 sm:grid-cols-2">
                    <CoverageCard title="Market structure" text={`${report.internal.token.buy_count} buys, ${report.internal.token.sell_count} sells, and ${report.internal.token.volume_bnb.toFixed(2)} BNB tracked in the current read.`} />
                    <CoverageCard title="Wallet structure" text={`${compactNumber(participantWalletCount(report))} participant wallets, ${report.internal.wallet_structure.probable_cluster_wallets} probable cluster wallets, and concentration checks attached.`} />
                    <CoverageCard title="Builder history" text={report.internal.deployer_memory ? `${report.internal.deployer_memory.total_launches} launches tracked with trust grade ${report.internal.deployer_memory.trust_grade}.` : 'No deployer profile is attached yet, so this layer is lighter.'} />
                    <CoverageCard title="Narrative layer" text={report.market_intelligence.active_event ? `Active topic detected: ${report.market_intelligence.active_event}.` : 'Narrative feeds are currently muted for this token.'} />
                    <CoverageCard title="Proof layer" text={outcomes.length > 0 ? `${outcomes.length} recent verified outcomes are attached for replay context.` : 'Replay outcomes are not populated on this deployment yet.'} />
                    <CoverageCard title="Source health" text={report.market_intelligence.sources.length > 0 ? `${report.market_intelligence.sources.length} attached citations are available to inspect.` : 'No citations were attached to this investigation yet.'} />
                  </div>
                </Panel>
              </section>
            )}

            {showDetail && activeSection === 'layers' && (
              <>
                <section className="grid gap-6 lg:grid-cols-[1.05fr_0.95fr]">
                  <Panel title="How MIA Read This Token" icon={<FaChartLine size={14} />}>
                    <div className="space-y-3 text-sm">
                      <Row label="Token" value={`${resolvedToken?.symbol ?? report.internal.token.symbol ?? 'UNKNOWN'} • ${resolvedToken?.name ?? report.internal.token.name ?? shortAddress(report.token_address)}`} />
                      <Row label="Contract" value={shortAddress(report.internal.token.contract_address, 10, 6)} />
                      <Row label="Builder" value={shortAddress(report.internal.token.deployer_address, 10, 6)} />
                      <Row label="Investigation score" value={displayInvestigationScore(report)} />
                      <Row label="Current volume" value={`${report.internal.token.volume_bnb.toFixed(2)} BNB`} />
                      <Row label="Participant wallets" value={compactNumber(participantWalletCount(report))} />
                      <Row label="Indexed holders" value={compactNumber(indexedHolderCount(report))} />
                      <Row label="Operator pattern" value={report.internal.operator_family.confidence} />
                    </div>

                    <div className="mt-5 space-y-3">
                      {visibleLayerCards.map((layer) => (
                        <LayerAccordion
                          key={layer.id}
                          layer={layer}
                          open={Boolean(openLayers[layer.id])}
                          readMode={readMode}
                          onToggle={() => toggleLayer(layer.id)}
                        />
                      ))}
                    </div>

                    {viewMode === 'quick' && layerCards.length > visibleLayerCards.length && (
                      <p className="mt-4 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                        Detailed View opens the remaining layers, including deeper builder and proof context.
                      </p>
                    )}
                  </Panel>

                  <Panel title="Field Notes" icon={<FaGlobe size={14} />}>
                    <div className="rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}>
                      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                        Proof support layer
                      </p>
                      <p className="mt-2 text-sm leading-7">{mlSummary}</p>
                    </div>
                    <NarrativeBlock title="Web summary" text={report.market_intelligence.web_summary ?? 'No web or news summary returned.'} />
                    <NarrativeBlock title="X summary" text={report.market_intelligence.x_summary ?? 'No realtime X summary returned.'} />
                    <div className="mt-4 grid gap-3 sm:grid-cols-2">
                      <InfoPill
                        label="Active topic"
                        value={report.market_intelligence.active_event ?? 'None detected'}
                        note={report.market_intelligence.narrative_alignment ?? report.market_intelligence.provider}
                      />
                      <InfoPill
                        label="Operator summary"
                        value={report.internal.operator_family.confidence}
                        note={report.internal.operator_family.summary}
                      />
                    </div>
                  </Panel>
                </section>

                {viewMode === 'full' && (
                  <section className="grid gap-6 lg:grid-cols-3">
                    <Panel title="Builder History" icon={<FaUserSecret size={14} />}>
                      {report.internal.deployer_memory ? (
                        <div className="space-y-3 text-sm">
                          <Row label="Trust grade" value={`${report.internal.deployer_memory.trust_grade} • ${report.internal.deployer_memory.trust_label}`} />
                          <Row label="Total launches" value={String(report.internal.deployer_memory.total_launches)} />
                          <Row label="Graduates" value={String(report.internal.deployer_memory.graduated_count)} />
                          <Row label="Rugs" value={String(report.internal.deployer_memory.rug_count)} />
                          <Row label="First seen" value={formatDate(report.internal.deployer_memory.first_seen_at)} />
                        </div>
                      ) : (
                        <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                          No deployer profile is currently available in the indexed dataset.
                        </p>
                      )}

                      {report.internal.deployer_recent_tokens.length > 0 && (
                        <div className="mt-5 space-y-2">
                          {report.internal.deployer_recent_tokens.slice(0, 4).map((token) => (
                            <Link
                              key={token.contract_address}
                              href={`/mia?q=${encodeURIComponent(token.contract_address)}`}
                              className="flex items-center justify-between rounded-lg border px-3 py-2 text-xs transition-colors hover:border-[rgba(105,137,255,0.35)]"
                              style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}
                            >
                              <div>
                                <p className="font-bold">{token.symbol ?? token.name ?? shortAddress(token.contract_address)}</p>
                                <p style={{ color: 'var(--on-surface-variant)' }}>{formatDate(token.deployed_at)}</p>
                              </div>
                              <span className="mono">{token.composite_score ?? 'n/a'}</span>
                            </Link>
                          ))}
                        </div>
                      )}
                    </Panel>

                    <Panel title="Narrative Layer" icon={<FaGlobe size={14} />}>
                      <div className="space-y-3 text-sm">
                        <Row label="Provider" value={report.market_intelligence.provider} />
                        <Row label="Narrative state" value={report.market_intelligence.narrative_alignment ?? 'n/a'} />
                        <Row label="Active topic" value={report.market_intelligence.active_event ?? 'none detected'} />
                      </div>
                      <NarrativeBlock title="Web summary" text={report.market_intelligence.web_summary ?? 'No web or news summary returned.'} />
                      <NarrativeBlock title="X summary" text={report.market_intelligence.x_summary ?? 'No realtime X summary returned.'} />
                      <ListBlock title="Public risk flags" items={report.market_intelligence.risk_flags} empty="No public-narrative risk flags returned." tone="danger" compact />
                    </Panel>

                    <Panel title="Wallet Structure" icon={<FaShieldHalved size={14} />}>
                      <div className="space-y-3 text-sm">
                        <Row label="Participant wallets" value={compactNumber(participantWalletCount(report))} />
                        <Row label="Indexed holders" value={compactNumber(indexedHolderCount(report))} />
                        <Row label="Probable clusters" value={String(report.internal.wallet_structure.probable_cluster_wallets)} />
                        <Row label="Repeated wallets" value={String(report.internal.wallet_structure.repeated_wallet_count)} />
                        <Row label="Operator confidence" value={report.internal.operator_family.confidence} />
                        <Row label="Related launches" value={String(report.internal.operator_family.related_launch_count)} />
                        <Row label="Top-holder provider" value={report.contract_intelligence.provider} />
                      </div>

                      <div className="mt-5 rounded-xl border p-4 text-sm" style={{ background: 'rgba(255,186,73,0.06)', borderColor: 'rgba(255,186,73,0.16)' }}>
                        <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                          Operator pattern summary
                        </p>
                        <p className="mt-2 leading-7">{report.internal.operator_family.summary}</p>
                      </div>

                      {report.contract_intelligence.top_holders.length > 0 && (
                        <div className="mt-5">
                          <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                            Top 50 holders
                          </p>
                          <p className="mb-3 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                            MIA highlights the owner inside the holder map when the deployer address appears in the latest top-50 holder list returned by the provider.
                          </p>
                          <div
                            className="overflow-x-auto rounded-xl border"
                            style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}
                          >
                            <div className="min-w-[720px]">
                              <div
                                className="grid grid-cols-[56px_minmax(220px,1.4fr)_110px_minmax(160px,1fr)_140px] gap-3 border-b px-4 py-3 text-[10px] font-bold uppercase tracking-[0.2em]"
                                style={{ borderColor: 'rgba(141,144,161,0.12)', color: 'var(--outline)' }}
                              >
                                <span>Rank</span>
                                <span>Wallet</span>
                                <span>Share</span>
                                <span>Amount</span>
                                <span>Tag</span>
                              </div>
                              <div className="max-h-[34rem] overflow-y-auto">
                                {report.contract_intelligence.top_holders.map((holder, index) => (
                                  <div
                                    key={`${holder.address}-${holder.quantity_raw}-${index}`}
                                    className="grid grid-cols-[56px_minmax(220px,1.4fr)_110px_minmax(160px,1fr)_140px] gap-3 border-b px-4 py-3 text-xs last:border-none"
                                    style={{ borderColor: 'rgba(141,144,161,0.08)' }}
                                  >
                                    <span className="mono">{index + 1}</span>
                                    <div className="space-y-1">
                                      <p className="mono font-semibold">{shortAddress(holder.address, 10, 6)}</p>
                                      <p style={{ color: 'var(--on-surface-variant)' }}>{holder.address}</p>
                                    </div>
                                    <span className="mono font-semibold">
                                      {formatPercent(holder.ownership_pct)}
                                    </span>
                                    <div className="space-y-1">
                                      <p className="mono font-semibold">{holder.quantity}</p>
                                      <p style={{ color: 'var(--on-surface-variant)' }}>{holder.quantity_raw} raw</p>
                                    </div>
                                    <div>
                                      <span
                                        className="inline-flex rounded-full px-2.5 py-1 text-[10px] font-bold uppercase tracking-[0.18em]"
                                        style={
                                          holder.is_owner
                                            ? { color: 'var(--danger)', background: 'rgba(255,107,107,0.14)' }
                                            : { color: 'var(--primary)', background: 'rgba(105,137,255,0.12)' }
                                        }
                                      >
                                        {holderBadge(holder)}
                                      </span>
                                    </div>
                                  </div>
                                ))}
                              </div>
                            </div>
                          </div>
                        </div>
                      )}

                      {report.contract_intelligence.top_holders.length === 0 && (
                        <p className="mt-5 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                          Top-holder data is not available from the current provider for this token yet.
                        </p>
                      )}
                    </Panel>
                  </section>
                )}
              </>
            )}

            {showDetail && activeSection === 'timeline' && (
              <section className="grid gap-6 lg:grid-cols-[1.05fr_0.95fr]">
                <Panel title="Run Timeline" icon={<FaDatabase size={14} />}>
                  <div
                    data-testid="mia-run-console-change-note"
                    className="rounded-xl border p-4"
                    style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}
                  >
                    <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                      What changed since last update
                    </p>
                    <p className="mt-2 text-base font-semibold">{runChangeSummary.headline}</p>
                    <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                      {runChangeSummary.detail}
                    </p>
                  </div>

                  {runDetailError && (
                    <div
                      className="mt-4 rounded-xl border p-4 text-sm"
                      style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}
                    >
                      {runDetailError}
                    </div>
                  )}

                  <div
                    data-testid="mia-run-console-timeline"
                    className="mt-5 space-y-3"
                  >
                    {(runDetail?.timeline ?? []).length > 0 ? (
                      runDetail!.timeline.map((event) => (
                        <TimelineEventRow key={`${event.key}-${event.at}`} event={event} />
                      ))
                    ) : (
                      <div
                        className="rounded-xl border p-4 text-sm"
                        style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)', color: 'var(--on-surface-variant)' }}
                      >
                        The run timeline will appear here once the run-detail payload is available.
                      </div>
                    )}
                  </div>
                </Panel>

                <Panel title="Run Console Notes" icon={<FaWaveSquare size={14} />}>
                  <div className="space-y-3 text-sm">
                    <Row label="Run ID" value={currentRun ? shortAddress(currentRun.run_id, 8, 6) : 'n/a'} />
                    <Row label="Trigger" value={currentRun ? formatRunTriggerLabel(currentRun.trigger_type) : 'n/a'} />
                    <Row label="Status" value={currentRun?.status ?? 'n/a'} />
                    <Row label="Stage" value={formatStageLabel(currentRun?.current_stage)} />
                    <Row label="Signal" value={signalTagLabel(currentRun?.signal_tag)} />
                    <Row label="Current read" value={currentRun?.current_read ?? 'n/a'} />
                    <Row label="Updated" value={currentRun ? `${formatRelativeTime(currentRun.updated_at)} • ${formatDate(currentRun.updated_at)}` : 'n/a'} />
                    <Row label="Status reason" value={currentRun?.status_reason ?? 'n/a'} />
                    <Row label="Evidence delta" value={currentRun?.evidence_delta ?? 'n/a'} />
                  </div>

                  <div className="mt-5 grid gap-3 sm:grid-cols-2">
                    <CoverageCard
                      title="Continuity"
                      text={runDetail?.continuity_note ?? 'This token is attached to a persistent run and can now be reopened through the same object path.'}
                    />
                    <CoverageCard
                      title="Run surface"
                      text="Use Overview for summary, Timeline for state changes, Sources for provenance, and Tools only when you want follow-through actions."
                    />
                  </div>
                </Panel>
              </section>
            )}

            {showDetail && activeSection === 'proof' && (
              <section className="grid gap-6 lg:grid-cols-[1.05fr_0.95fr]">
                <Panel title="Proof Layer" icon={<FaWaveSquare size={14} />}>
                  <div className="rounded-xl border p-4" style={{ background: 'rgba(0,255,163,0.06)', borderColor: 'rgba(0,255,163,0.14)' }}>
                    <div className="inline-flex rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.1)' }}>
                      {proofBadge}
                    </div>
                    <p className="mt-3 text-base font-semibold">{proofHeadline}</p>
                    <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                      {proofSupport}
                    </p>
                  </div>

                  <div className="mt-4 rounded-xl border p-4" style={{ background: 'rgba(105,137,255,0.05)', borderColor: 'rgba(105,137,255,0.12)' }}>
                    <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                      Machine-learning read
                    </p>
                    <p className="mt-2 text-sm leading-7">{mlSummary}</p>
                  </div>

                  <div className="mt-5 flex flex-wrap gap-2">
                    <Link
                      href="/backtesting"
                      className="rounded-lg border px-3 py-2 text-xs font-semibold transition-colors hover:border-[rgba(105,137,255,0.35)]"
                      style={{ background: 'var(--surface-container-lowest)', color: 'var(--primary)', borderColor: 'rgba(141,144,161,0.12)' }}
                    >
                      Open Replay Lab
                    </Link>
                    <Link
                      href="/alpha"
                      className="rounded-lg border px-3 py-2 text-xs font-semibold transition-colors hover:border-[rgba(105,137,255,0.35)]"
                      style={{ background: 'var(--surface-container-lowest)', color: 'var(--primary)', borderColor: 'rgba(141,144,161,0.12)' }}
                    >
                      Open Live Alpha
                    </Link>
                  </div>
                </Panel>

                <Panel title="Recent Verified Outcomes" icon={<FaChartLine size={14} />}>
                  <p className="text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                    This panel shows whether replay evidence and recent evaluations are available, so users can separate grounded support from pure narrative confidence.
                  </p>

                  <div className="mt-5">
                    {outcomes.length > 0 ? (
                      <div className="space-y-3">
                        {outcomes.map((outcome) => (
                          <OutcomeRow key={`${outcome.tokenAddress}-${outcome.score}`} outcome={outcome} />
                        ))}
                      </div>
                    ) : (
                      <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                        Replay rows are not attached on this deployment yet. The investigation still resolves the token, but the proof layer needs historical samples to populate.
                      </p>
                    )}
                  </div>
                </Panel>
              </section>
            )}

            {showDetail && activeSection === 'sources' && (
              <section className="grid gap-6 lg:grid-cols-[0.9fr_1.1fr]">
                <Panel title="Transaction Trail" icon={<FaWaveSquare size={14} />}>
                  <div className="space-y-3 text-sm">
                    <Row label="Whale alerts 24H" value={`${report.internal.whale_activity_24h.critical_alerts} critical / ${report.internal.whale_activity_24h.watch_alerts} watch`} />
                    <Row label="Support rank" value={report.internal.alpha_context ? `#${report.internal.alpha_context.rank} • ${report.internal.alpha_context.alpha_score.toFixed(1)}` : 'n/a'} />
                    <Row label="Holder provider" value={report.contract_intelligence.provider} />
                    <Row label="Generated" value={formatDate(report.generated_at)} />
                  </div>

                  <div className="mt-5 space-y-2">
                    {report.internal.recent_transactions.slice(0, viewMode === 'quick' ? 4 : 6).map((tx) => (
                      <div
                        key={tx.tx_hash}
                        className="flex items-center justify-between rounded-lg border px-3 py-2 text-xs"
                        style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}
                      >
                        <div>
                          <p className="font-bold uppercase">{tx.tx_type}</p>
                          <p style={{ color: 'var(--on-surface-variant)' }}>{shortAddress(tx.wallet_address, 8, 4)}</p>
                        </div>
                        <span className="mono">{tx.amount_bnb.toFixed(3)} BNB</span>
                      </div>
                    ))}
                  </div>
                </Panel>

                <Panel title="Sources, Fallbacks, and Tooling" icon={<FaLink size={14} />}>
                  <div className="mb-4 flex flex-wrap gap-2 text-[11px] uppercase tracking-widest">
                    <StatusPill label={`BscScan ${report.source_status.bscscan_configured ? 'connected' : 'not configured'}`} tone={report.source_status.bscscan_configured ? 'safe' : 'warn'} />
                    <StatusPill label={`Market source ${report.source_status.market_provider}`} tone="primary" />
                  </div>

                  {report.market_intelligence.sources.length > 0 ? (
                    <div className="space-y-2">
                      {report.market_intelligence.sources.slice(0, viewMode === 'quick' ? 6 : 8).map((source) => (
                        <a
                          key={`${source.url}-${source.title}`}
                          href={source.url}
                          target="_blank"
                          rel="noreferrer"
                          className="block rounded-lg border px-3 py-3 text-sm transition-colors hover:border-[rgba(105,137,255,0.35)]"
                          style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}
                        >
                          <p className="font-semibold">{source.title}</p>
                          <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                            {source.source}
                          </p>
                        </a>
                      ))}
                    </div>
                  ) : (
                    <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                      No citations were attached to this investigation yet.
                    </p>
                  )}

                  {report.source_status.notes.length > 0 && (
                    <div className="mt-5 rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.04)', borderColor: 'rgba(141,144,161,0.12)' }}>
                      <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                        Source notes
                      </p>
                      <div className="space-y-2 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                        {report.source_status.notes.slice(0, 6).map((note) => (
                          <div key={note} className="flex items-start gap-2">
                            <FaTriangleExclamation className="mt-0.5 shrink-0" size={10} />
                            <span>{note}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </Panel>
              </section>
            )}

            {showDetail && activeSection === 'tools' && (
              <>
                <AskMiaEntryCard
                  tokenAddress={report.internal.token.contract_address}
                  tokenLabel={resolvedToken?.symbol ?? resolvedToken?.name ?? report.internal.token.symbol ?? shortAddress(report.token_address)}
                  runId={currentRun?.run_id ?? null}
                />

                <Panel title="Optional Monitoring and Route Tools" icon={<FaBolt size={14} />}>
                  <div className="rounded-xl border p-4" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}>
                    <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                      Linked investigation tools
                    </p>
                    <p className="mt-2 text-base font-semibold">{executionPlan.stance}</p>
                    <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                      This lane keeps the optional support tools together: external routes, watch setup, and the copyable notes. The main investigation surface stays evidence-first above.
                    </p>
                  </div>

                  <div className="mt-5 flex flex-wrap gap-3">
                    {executionPlan.primaryActions.map((action, index) => (
                      <ActionLinkButton
                        key={`${action.label}-${action.href}`}
                        action={action}
                        variant={index === 0 ? 'hero' : 'secondary'}
                        caption={
                          index === 0
                          ? 'Primary route'
                          : action.label.toLowerCase().includes('alert')
                            ? 'Monitoring'
                              : 'Linked route'
                        }
                      />
                    ))}
                  </div>

                  <div className="mt-3 flex flex-wrap gap-2">
                    {executionPlan.supportingActions.map((action) => (
                      <ActionLinkButton key={`${action.label}-${action.href}`} action={action} variant="ghost" />
                    ))}
                  </div>

                  <div className="mt-5 grid gap-4 md:grid-cols-3">
                    <ActionPlanCard title="Access note" text={executionPlan.entryPlan} />
                    <ActionPlanCard title="Risk response" text={executionPlan.exitPlan} />
                    <ActionPlanCard title="Monitoring plan" text={executionPlan.alertPlan} />
                  </div>

                  {actionBrief && (
                    <div className="mt-5 rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.14)' }}>
                      <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                        <div>
                          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                            Investigation brief
                          </p>
                          <p className="mt-2 text-base font-semibold">{actionBrief.headline}</p>
                          <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                            {actionBrief.summary}
                          </p>
                        </div>
                        <button
                          onClick={copyActionBrief}
                          className="rounded-lg border px-3 py-2 text-[11px] font-bold uppercase tracking-[0.16em] transition-colors hover:border-[rgba(105,137,255,0.35)]"
                          style={{ background: 'rgba(105,137,255,0.14)', color: 'var(--primary)', borderColor: 'rgba(105,137,255,0.18)' }}
                        >
                          {briefCopied ? 'Copied' : 'Copy Notes'}
                        </button>
                      </div>

                      <div className="mt-4 grid gap-2 md:grid-cols-2">
                        {actionBrief.checklist.map((item) => (
                          <div
                            key={item}
                            className="rounded-lg border px-3 py-2 text-sm"
                            style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'rgba(255,255,255,0.02)' }}
                          >
                            {item}
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  <div className="mt-5">
                    <ListBlock title="Why the current read leans this way" items={executionPlan.reasons} empty="No supporting notes returned." />
                  </div>
                </Panel>

                <DeepResearchPanels
                  tokenAddress={report.internal.token.contract_address}
                  preview={deepResearchPreview}
                  status={deepResearchStatus}
                  report={deepResearchReport}
                  entitlementToken={deepResearchEntitlement}
                  loading={deepResearchLoading}
                  error={deepResearchError}
                />
              </>
            )}
          </>
        )}
        </div>

        <div className="mx-auto hidden w-full max-w-[1420px] lg:block">
          <div
            className="mb-5 flex items-center gap-4 rounded-[22px] border px-5 py-4"
            style={{ background: 'rgba(18,24,38,0.9)', borderColor: 'rgba(111,141,255,0.2)' }}
          >
            <FaMagnifyingGlass size={18} color="#6f8dff" />
            <input
              value={input}
              onChange={(event) => setInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') runInvestigation(input);
              }}
              className="w-full border-none bg-transparent text-sm focus:ring-0"
              style={{ color: '#edf1ff', fontFamily: 'Space Grotesk, sans-serif' }}
              placeholder="Search ticker or contract address..."
            />
            <Link
              href="/mia/runs"
              className="rounded-[14px] border px-4 py-3 text-[11px] font-extrabold uppercase tracking-[0.14em]"
              style={{ color: '#adb6d0', borderColor: 'rgba(255,255,255,0.08)', background: 'rgba(255,255,255,0.04)', fontFamily: 'Manrope, sans-serif' }}
            >
              Open Runs
            </Link>
            <button
              onClick={() => runInvestigation(input)}
              disabled={loading}
              className="rounded-[14px] px-5 py-3 text-[11px] font-extrabold uppercase tracking-[0.14em] disabled:opacity-50"
              style={{ background: '#6f8dff', color: '#081736', fontFamily: 'Manrope, sans-serif' }}
            >
              {loading ? 'Opening...' : 'Investigate'}
            </button>
          </div>

          {loading && !report && (
            <div className="rounded-[24px] border px-10 py-16 text-center" style={{ background: 'rgba(18,24,38,0.9)', borderColor: 'rgba(111,141,255,0.16)' }}>
              <div className="text-[11px] font-bold uppercase tracking-[0.18em]" style={{ color: '#6f8dff', fontFamily: 'Manrope, sans-serif' }}>
                MIA ANALYZING
              </div>
              <p className="mt-3 text-base" style={{ color: '#94a0c2', fontFamily: 'Space Grotesk, sans-serif' }}>
                Building the investigation screen from on-chain, builder, wallet, and evidence layers.
              </p>
            </div>
          )}

          {error && (
            <section
              className="rounded-2xl border p-5 text-sm"
              style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.24)', color: 'var(--danger)' }}
            >
              {error}
            </section>
          )}

          {!report && !loading && !error && (
            <div className="grid gap-6 xl:grid-cols-[280px_minmax(0,1fr)_320px]">
              <aside className="space-y-4">
                <div className="rounded-[24px] border p-6" style={{ background: 'rgba(18,24,38,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}>
                  <div className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: '#94a0c2', fontFamily: 'Manrope, sans-serif' }}>
                    Investigation Workspace
                  </div>
                  <div className="mt-3 text-[28px] font-black leading-[1.02]" style={{ color: '#edf1ff', fontFamily: 'Manrope, sans-serif' }}>
                    Open one token. Get one clear screen.
                  </div>
                  <p className="mt-3 text-sm leading-7" style={{ color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif' }}>
                    Use MIA when you need a calm read first, then deeper evidence only if the token deserves it.
                  </p>
                </div>

                <div className="rounded-[24px] border p-5" style={{ background: 'rgba(23,31,49,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}>
                  <div className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: '#94a0c2', fontFamily: 'Manrope, sans-serif', marginBottom: 12 }}>
                    Coverage
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {['Flow', 'Wallets', 'Builder', 'Narrative', 'Proof'].map((chip, index) => (
                      <span
                        key={chip}
                        className="rounded-full border px-3 py-1 text-[10px] font-bold uppercase tracking-[0.12em]"
                        style={{
                          color: index === 1 ? '#36efb6' : index === 2 ? '#ffd166' : index === 4 ? '#a8b8ff' : '#94a0c2',
                          borderColor: 'rgba(255,255,255,0.08)',
                          background: 'rgba(255,255,255,0.04)',
                        }}
                      >
                        {chip}
                      </span>
                    ))}
                  </div>
                </div>
              </aside>

              <section className="rounded-[28px] border p-8 text-center" style={{ background: 'linear-gradient(135deg, rgba(111,141,255,0.12), rgba(18,24,38,0.96))', borderColor: 'rgba(111,141,255,0.18)' }}>
                <div style={{ fontSize: 52, marginBottom: 16 }}>🔎</div>
                <h1 className="font-headline text-4xl font-extrabold tracking-tight" data-testid="mia-page-heading">
                  Start an investigation.
                </h1>
                <p className="mx-auto mt-4 max-w-[36rem] text-base leading-8" style={{ color: '#94a0c2', fontFamily: 'Space Grotesk, sans-serif' }}>
                  Search a ticker or paste a contract address. MIA opens a focused screen first, then lets you decide whether you want a quick read or a deeper research path.
                </p>
              </section>

              <aside className="space-y-4">
                <div className="rounded-[24px] border p-5" style={{ background: 'rgba(23,31,49,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}>
                  <div className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: '#94a0c2', fontFamily: 'Manrope, sans-serif', marginBottom: 10 }}>
                    Entry Modes
                  </div>
                  <div className="space-y-3">
                    <div className="rounded-2xl border p-4" style={{ background: 'rgba(111,141,255,0.08)', borderColor: 'rgba(111,141,255,0.18)' }}>
                      <div className="text-[11px] font-extrabold uppercase tracking-[0.12em]" style={{ color: '#6f8dff', fontFamily: 'Manrope, sans-serif' }}>
                        Quick Read
                      </div>
                      <p className="mt-2 text-sm leading-7" style={{ color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif' }}>
                        Fast evidence summary, signals, and current read without opening the heavy layers.
                      </p>
                    </div>
                    <div className="rounded-2xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(255,255,255,0.08)' }}>
                      <div className="text-[11px] font-extrabold uppercase tracking-[0.12em]" style={{ color: '#edf1ff', fontFamily: 'Manrope, sans-serif' }}>
                        Deep Research
                      </div>
                      <p className="mt-2 text-sm leading-7" style={{ color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif' }}>
                        Open the full route when the token deserves source depth, extended panels, and longer continuity.
                      </p>
                    </div>
                  </div>
                </div>
              </aside>
            </div>
          )}

          {report && executionPlan && (
            <>
              <div className="grid gap-6 xl:grid-cols-[270px_minmax(0,1fr)_340px]">
                <aside className="space-y-4">
                  <div className="rounded-[24px] border p-6" style={{ background: 'rgba(18,24,38,0.92)', borderColor: 'rgba(255,255,255,0.06)' }}>
                    <div className="flex items-center gap-3">
                      <ScoreOrb
                        score={decisionScorecard(report).decision_score ?? 0}
                        tone={subscoreTone(decisionScorecard(report).decision_score ?? 32)}
                        label="MIA"
                      />
                      <div>
                        <div style={{ fontSize: 22, fontWeight: 900, color: '#edf1ff', fontFamily: 'Manrope, sans-serif' }}>
                          {resolvedToken?.symbol ?? report.internal.token.symbol ?? 'TOKEN'}
                        </div>
                        <div style={{ marginTop: 4, fontSize: 11, color: '#94a0c2', fontFamily: 'Roboto Mono, monospace' }}>
                          {shortAddress(report.internal.token.contract_address, 10, 6)}
                        </div>
                      </div>
                    </div>

                    <div className="mt-5 flex flex-wrap gap-2">
                      <span
                        className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.14em]"
                        style={{
                          color:
                            report.internal.risk?.risk_category === 'high'
                              ? '#ff8080'
                              : report.internal.risk?.risk_category === 'low'
                                ? '#36efb6'
                                : '#ffd166',
                          background:
                            report.internal.risk?.risk_category === 'high'
                              ? 'rgba(255,128,128,0.12)'
                              : report.internal.risk?.risk_category === 'low'
                                ? 'rgba(54,239,182,0.1)'
                                : 'rgba(255,209,102,0.1)',
                        }}
                      >
                        {(report.internal.risk?.risk_category ?? 'medium').toUpperCase()} risk
                      </span>
                      {deepResearchBadge(report) ? (
                        <span
                          className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.14em]"
                          style={{ color: '#6f8dff', background: 'rgba(111,141,255,0.12)' }}
                        >
                          {deepResearchBadge(report)}
                        </span>
                      ) : null}
                      {currentRun?.signal_tag ? <SignalBadge value={currentRun.signal_tag} /> : null}
                    </div>

                    <div className="mt-5 grid gap-3">
                      <button
                        type="button"
                        data-testid="mia-intent-quick"
                        onClick={() => {
                          setViewMode('quick');
                          setReadMode('human');
                          setActiveSection('overview');
                        }}
                        style={{
                          width: '100%',
                          padding: '13px 16px',
                          borderRadius: 16,
                          background: '#6f8dff',
                          border: 'none',
                          color: '#081736',
                          fontSize: 12,
                          fontWeight: 800,
                          cursor: 'pointer',
                          letterSpacing: '0.12em',
                          textTransform: 'uppercase',
                          fontFamily: 'Manrope, sans-serif',
                        }}
                      >
                        Quick Read
                      </button>
                      <button
                        type="button"
                        data-testid="mia-intent-deep"
                        onClick={() => {
                          setViewMode('full');
                          setReadMode('analyst');
                          setActiveSection('tools');
                          setShowDetail(true);
                        }}
                        style={{
                          width: '100%',
                          padding: '13px 16px',
                          borderRadius: 16,
                          background: 'rgba(255,255,255,0.05)',
                          border: '1px solid rgba(255,255,255,0.1)',
                          color: '#adb6d0',
                          fontSize: 12,
                          fontWeight: 800,
                          cursor: 'pointer',
                          letterSpacing: '0.12em',
                          textTransform: 'uppercase',
                          fontFamily: 'Manrope, sans-serif',
                        }}
                      >
                        Deep Research
                      </button>
                    </div>
                  </div>

                  <div className="rounded-[24px] border p-5" style={{ background: 'rgba(23,31,49,0.9)', borderColor: 'rgba(255,255,255,0.06)' }}>
                    <div className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: '#94a0c2', fontFamily: 'Manrope, sans-serif', marginBottom: 12 }}>
                      Investigation Lanes
                    </div>
                    <div className="grid gap-3">
                      {workspaceSections.map((section) => (
                        <WorkspaceSectionButton
                          key={section.value}
                          section={section.value}
                          label={section.label}
                          description={section.description}
                          icon={section.icon}
                          active={activeSection === section.value}
                          onClick={() => {
                            setShowDetail(true);
                            setActiveSection(section.value);
                          }}
                        />
                      ))}
                    </div>
                  </div>
                </aside>

                <section className="min-w-0">
                  <section
                    data-testid="mia-mobile-hero-card"
                    className="rounded-[28px] border p-7"
                    style={{
                      marginBottom: 18,
                      background: 'linear-gradient(135deg, rgba(111,141,255,0.14), rgba(12,16,24,0.98))',
                      borderColor: 'rgba(111,141,255,0.22)',
                    }}
                  >
                    <div className="flex items-center justify-between gap-6">
                      <div className="flex items-center gap-5">
                        <ScoreOrb
                          score={decisionScorecard(report).decision_score ?? 0}
                          tone={subscoreTone(decisionScorecard(report).decision_score ?? 32)}
                          label="MIA"
                        />
                        <div>
                          <div className="mb-3">
                            <span
                              style={{
                                background: summaryTone.background,
                                color: summaryTone.color,
                                border: `1px solid ${summaryTone.background}`,
                                borderRadius: 999,
                                padding: '8px 16px',
                                fontSize: 13,
                                fontWeight: 800,
                                letterSpacing: '0.12em',
                                textTransform: 'uppercase',
                                whiteSpace: 'nowrap',
                                fontFamily: 'Manrope, sans-serif',
                              }}
                            >
                              {plainVerdictLabel(decisionVerdict(report))}
                            </span>
                          </div>
                          <div style={{ fontSize: 30, lineHeight: 1.05, fontWeight: 900, color: '#edf1ff', fontFamily: 'Manrope, sans-serif', maxWidth: 520 }}>
                            {hasAgentScore(report)
                              ? `MIA Agent Score ${decisionScorecard(report).decision_score}`
                              : 'No score yet'}
                          </div>
                          <div style={{ marginTop: 8, fontSize: 14, color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif', lineHeight: 1.7, maxWidth: 580 }}>
                            {scoreStatusLine(report)}. {decisionSummary}
                          </div>
                        </div>
                      </div>
                    </div>
                  </section>

                  <div style={{ marginBottom: 18 }}>
                    <InvestigationScorePanel report={report} />
                  </div>

                  <div className="grid gap-5 xl:grid-cols-[minmax(0,1.05fr)_320px]">
                    <div>
                      <section
                        data-testid="mia-evidence-signals-card"
                        className="rounded-[24px] border p-6"
                        style={{
                          marginBottom: 16,
                          background: 'rgba(23,31,49,0.9)',
                          borderColor: 'rgba(255,255,255,0.06)',
                        }}
                      >
                        <div style={{ fontSize: 10, fontWeight: 800, color: '#94a0c2', letterSpacing: '0.16em', textTransform: 'uppercase', fontFamily: 'Manrope, sans-serif', marginBottom: 14 }}>
                          Evidence Signals
                        </div>
                        <div className="grid gap-4">
                          {visibleSignals.map((signal) => (
                            <EvidenceSignalRow
                              key={signal.label}
                              label={signal.label}
                              value={signal.value}
                              note={signal.note}
                              meter={signal.meter}
                              tone={signal.tone}
                            />
                          ))}
                        </div>
                      </section>

                      <section
                        className="rounded-[24px] border p-6"
                        style={{
                          marginBottom: 16,
                          background: 'rgba(23,31,49,0.9)',
                          borderColor: 'rgba(255,255,255,0.06)',
                        }}
                      >
                        <div style={{ fontSize: 10, fontWeight: 800, color: '#6f8dff', letterSpacing: '0.16em', textTransform: 'uppercase', fontFamily: 'Manrope, sans-serif', marginBottom: 10 }}>
                          ✦ MIA Analysis
                        </div>
                        <p style={{ margin: 0, fontSize: 15, color: '#adb6d0', fontFamily: 'Space Grotesk, sans-serif', lineHeight: 1.8 }}>
                          {decisionSummary}
                        </p>
                      </section>

                      <div>
                        <button
                          type="button"
                          onClick={() => setShowDetail((value) => !value)}
                          style={{
                            width: '100%',
                            padding: '13px 16px',
                            borderRadius: 16,
                            background: 'rgba(255,255,255,0.04)',
                            border: '1px solid rgba(255,255,255,0.08)',
                            color: '#94a0c2',
                            fontSize: 12,
                            fontWeight: 700,
                            cursor: 'pointer',
                            fontFamily: 'Manrope, sans-serif',
                            letterSpacing: '0.12em',
                            textTransform: 'uppercase',
                          }}
                        >
                          {showDetail ? 'Hide Details ↑' : 'Show More Details ↓'}
                        </button>
                      </div>
                    </div>

                    <aside className="space-y-4">
                      <Panel title="Snapshot" icon={<FaDatabase size={14} />}>
                        <div className="grid gap-3">
                          <InfoPill label="Volume" value={`${report.internal.token.volume_bnb.toFixed(2)} BNB`} note="Tracked in the current read" />
                          <InfoPill label="Buy / Sell" value={`${report.internal.token.buy_count} / ${report.internal.token.sell_count}`} note="Current visible flow" />
                          <InfoPill label="Participants" value={compactNumber(participantWalletCount(report))} note={`${compactNumber(indexedHolderCount(report))} indexed holders`} />
                          <InfoPill label="Builder" value={report.internal.deployer_memory?.trust_grade ?? 'n/a'} note={report.internal.deployer_memory?.summary ?? 'Builder memory is still thin on this token.'} />
                          <InfoPill label="Proof" value={proofBadge} note={proofHeadline} />
                        </div>
                      </Panel>

                      {currentRun ? (
                        <Panel title="Run Console" icon={<FaWaveSquare size={14} />}>
                          <div className="grid gap-3">
                            <RunConsoleChip label="Run type" value={formatRunTriggerLabel(currentRun.trigger_type)} note="Manual and auto runs share the same object." />
                            <RunConsoleChip label="What changed" value={runChangeSummary.headline} note={runChangeSummary.detail} />
                            <div className="grid gap-2">
                              <button type="button" onClick={() => applyRunStatus('watching')} disabled={runActionLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(105,137,255,0.18)', color: 'var(--primary)', background: 'rgba(105,137,255,0.08)' }}>
                                {runActionLoading === 'watching' ? 'Updating...' : 'Mark Watching'}
                              </button>
                              <button type="button" onClick={() => applyRunStatus('escalated')} disabled={runActionLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(255,186,73,0.2)', color: 'var(--warning)', background: 'rgba(255,186,73,0.08)' }}>
                                {runActionLoading === 'escalated' ? 'Updating...' : 'Escalate Run'}
                              </button>
                              <button type="button" onClick={() => applyRunStatus('archived')} disabled={runActionLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(141,144,161,0.18)', color: 'var(--outline)', background: 'rgba(255,255,255,0.05)' }}>
                                {runActionLoading === 'archived' ? 'Updating...' : 'Archive Run'}
                              </button>
                            </div>
                            <div className="grid gap-2">
                              <button type="button" onClick={() => saveWatchlistItem('token')} disabled={watchSaveLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(0,255,163,0.18)', color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }}>
                                {watchSaveLoading === 'token' ? 'Saving...' : 'Save Token Watch'}
                              </button>
                              <button type="button" onClick={() => saveWatchlistItem('builder')} disabled={watchSaveLoading !== null} className="rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em] disabled:opacity-50" style={{ borderColor: 'rgba(141,144,161,0.18)', color: 'var(--primary)', background: 'rgba(255,255,255,0.05)' }}>
                                {watchSaveLoading === 'builder' ? 'Saving...' : 'Save Builder Watch'}
                              </button>
                            </div>
                          </div>
                        </Panel>
                      ) : null}
                    </aside>
                  </div>
                </section>
              </div>

              {showDetail && (
                <div className="mt-6">
                  {activeSection === 'tools' && (
                    <div className="mb-6">
                      <AskMiaEntryCard
                        tokenAddress={report.internal.token.contract_address}
                        tokenLabel={resolvedToken?.symbol ?? resolvedToken?.name ?? report.internal.token.symbol ?? shortAddress(report.token_address)}
                        runId={currentRun?.run_id ?? null}
                      />
                    </div>
                  )}

                  {activeSection === 'overview' && (
                    <section className="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
                      <Panel title="What matters right now" icon={<FaBrain size={14} />}>
                        <div className="rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}>
                          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                            Short evidence read
                          </p>
                          <p className="mt-2 text-base leading-7">{decisionSummary}</p>
                        </div>
                        <div className="mt-4 grid gap-3 sm:grid-cols-2">
                          <InfoPill label="Operator pattern" value={report.internal.operator_family.confidence} note={report.internal.operator_family.summary} />
                          <InfoPill label="Builder memory" value={report.internal.deployer_memory?.trust_grade ?? 'n/a'} note={report.internal.deployer_memory?.summary ?? 'Builder history is still thin on this token.'} />
                          <InfoPill label="Proof support" value={report.internal.alpha_context ? `Rank #${report.internal.alpha_context.rank}` : 'No live rank'} note={mlSummary ?? 'Proof support is not attached yet.'} />
                          <InfoPill label="Proof status" value={proofBadge} note={proofSupport} />
                        </div>
                      </Panel>
                      <Panel title="Coverage at a glance" icon={<FaChartLine size={14} />}>
                        <div className="grid gap-3 sm:grid-cols-2">
                          <CoverageCard title="Market structure" text={`${report.internal.token.buy_count} buys, ${report.internal.token.sell_count} sells, and ${report.internal.token.volume_bnb.toFixed(2)} BNB tracked in the current read.`} />
                          <CoverageCard title="Wallet structure" text={`${compactNumber(participantWalletCount(report))} participant wallets, ${report.internal.wallet_structure.probable_cluster_wallets} probable cluster wallets, and concentration checks attached.`} />
                          <CoverageCard title="Builder history" text={report.internal.deployer_memory ? `${report.internal.deployer_memory.total_launches} launches tracked with trust grade ${report.internal.deployer_memory.trust_grade}.` : 'No deployer profile is attached yet, so this layer is lighter.'} />
                          <CoverageCard title="Narrative layer" text={report.market_intelligence.active_event ? `Active topic detected: ${report.market_intelligence.active_event}.` : 'Narrative feeds are currently muted for this token.'} />
                          <CoverageCard title="Proof layer" text={outcomes.length > 0 ? `${outcomes.length} recent verified outcomes are attached for replay context.` : 'Replay outcomes are not populated on this deployment yet.'} />
                          <CoverageCard title="Source health" text={report.market_intelligence.sources.length > 0 ? `${report.market_intelligence.sources.length} attached citations are available to inspect.` : 'No citations were attached to this investigation yet.'} />
                        </div>
                      </Panel>
                    </section>
                  )}

                  {activeSection === 'layers' && (
                    <section className="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
                      <Panel title="How MIA Read This Token" icon={<FaChartLine size={14} />}>
                        <div className="space-y-3 text-sm">
                          <Row label="Token" value={`${resolvedToken?.symbol ?? report.internal.token.symbol ?? 'UNKNOWN'} • ${resolvedToken?.name ?? report.internal.token.name ?? shortAddress(report.token_address)}`} />
                          <Row label="Contract" value={shortAddress(report.internal.token.contract_address, 10, 6)} />
                          <Row label="Builder" value={shortAddress(report.internal.token.deployer_address, 10, 6)} />
                          <Row label="Investigation score" value={displayInvestigationScore(report)} />
                          <Row label="Current volume" value={`${report.internal.token.volume_bnb.toFixed(2)} BNB`} />
                          <Row label="Participant wallets" value={compactNumber(participantWalletCount(report))} />
                          <Row label="Indexed holders" value={compactNumber(indexedHolderCount(report))} />
                          <Row label="Operator pattern" value={report.internal.operator_family.confidence} />
                        </div>
                        <div className="mt-5 space-y-3">
                          {visibleLayerCards.map((layer) => (
                            <LayerAccordion key={layer.id} layer={layer} open={Boolean(openLayers[layer.id])} readMode={readMode} onToggle={() => toggleLayer(layer.id)} />
                          ))}
                        </div>
                      </Panel>
                      <Panel title="Field Notes" icon={<FaGlobe size={14} />}>
                        <div className="rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}>
                          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                            Proof support layer
                          </p>
                          <p className="mt-2 text-sm leading-7">{mlSummary}</p>
                        </div>
                        <NarrativeBlock title="Web summary" text={report.market_intelligence.web_summary ?? 'No web or news summary returned.'} />
                        <NarrativeBlock title="X summary" text={report.market_intelligence.x_summary ?? 'No realtime X summary returned.'} />
                      </Panel>
                    </section>
                  )}

                  {activeSection === 'timeline' && (
                    <section className="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
                      <Panel title="Run Timeline" icon={<FaDatabase size={14} />}>
                        <div data-testid="mia-run-console-change-note" className="rounded-xl border p-4" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}>
                          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                            What changed since last update
                          </p>
                          <p className="mt-2 text-base font-semibold">{runChangeSummary.headline}</p>
                          <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                            {runChangeSummary.detail}
                          </p>
                        </div>
                        <div className="mt-4 space-y-3">
                          {runDetail?.timeline.map((event) => <TimelineEventRow key={`${event.key}-${event.at}`} event={event} />)}
                        </div>
                      </Panel>
                      <Panel title="Run Summary" icon={<FaWaveSquare size={14} />}>
                        {currentRun ? (
                          <div className="grid gap-3">
                            <RunConsoleChip label="Run type" value={formatRunTriggerLabel(currentRun.trigger_type)} note="How this investigation entered the system." />
                            <RunConsoleChip label="Current stage" value={formatStageLabel(currentRun.current_stage)} note="Current stage inside the run lifecycle." />
                            <RunConsoleChip label="Last update" value={formatRelativeTime(currentRun.updated_at)} note="The run journal updates as the state changes." />
                          </div>
                        ) : (
                          <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                            No active run is attached to this screen yet.
                          </p>
                        )}
                      </Panel>
                    </section>
                  )}

                  {activeSection === 'proof' && (
                    <section className="grid gap-6 xl:grid-cols-[1fr_0.95fr]">
                      <Panel title="Proof Layer" icon={<FaShieldHalved size={14} />}>
                        <div className="grid gap-3 md:grid-cols-2">
                          {outcomes.length > 0 ? outcomes.map((outcome) => <OutcomeRow key={outcome.tokenAddress} outcome={outcome} />) : <CoverageCard title="Proof layer" text="Replay outcomes are not populated on this deployment yet." />}
                        </div>
                      </Panel>
                      <Panel title="Proof Snapshot" icon={<FaChartLine size={14} />}>
                        <div className="grid gap-3 sm:grid-cols-2">
                          <InfoPill label="Badge" value={proofBadge} note={proofHeadline} />
                          <InfoPill label="Replay support" value={proofSupport} note="Backtesting and proof context that support the current read." />
                        </div>
                      </Panel>
                    </section>
                  )}

                  {activeSection === 'sources' && (
                    <section className="grid gap-6 xl:grid-cols-[0.9fr_1.1fr]">
                      <Panel title="Transaction Trail" icon={<FaWaveSquare size={14} />}>
                        <div className="space-y-3 text-sm">
                          <Row label="Whale alerts 24H" value={`${report.internal.whale_activity_24h.critical_alerts} critical / ${report.internal.whale_activity_24h.watch_alerts} watch`} />
                    <Row label="Support rank" value={report.internal.alpha_context ? `#${report.internal.alpha_context.rank} • ${report.internal.alpha_context.alpha_score.toFixed(1)}` : 'n/a'} />
                    <Row label="Indexed holders" value={compactNumber(indexedHolderCount(report))} />
                    <Row label="Generated" value={formatDate(report.generated_at)} />
                        </div>
                        <div className="mt-5 space-y-2">
                          {report.internal.recent_transactions.slice(0, viewMode === 'quick' ? 4 : 6).map((tx) => (
                            <div key={tx.tx_hash} className="flex items-center justify-between rounded-lg border px-3 py-2 text-xs" style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}>
                              <div>
                                <p className="font-bold uppercase">{tx.tx_type}</p>
                                <p style={{ color: 'var(--on-surface-variant)' }}>{shortAddress(tx.wallet_address, 8, 4)}</p>
                              </div>
                              <span className="mono">{tx.amount_bnb.toFixed(3)} BNB</span>
                            </div>
                          ))}
                        </div>
                      </Panel>
                      <Panel title="Sources, Fallbacks, and Tooling" icon={<FaLink size={14} />}>
                        <div className="mb-4 flex flex-wrap gap-2 text-[11px] uppercase tracking-widest">
                          <StatusPill label={`BscScan ${report.source_status.bscscan_configured ? 'connected' : 'not configured'}`} tone={report.source_status.bscscan_configured ? 'safe' : 'warn'} />
                          <StatusPill label={`Market source ${report.source_status.market_provider}`} tone="primary" />
                        </div>
                        {report.market_intelligence.sources.length > 0 ? (
                          <div className="space-y-2">
                            {report.market_intelligence.sources.slice(0, viewMode === 'quick' ? 6 : 8).map((source) => (
                              <a key={`${source.url}-${source.title}`} href={source.url} target="_blank" rel="noreferrer" className="block rounded-lg border px-3 py-3 text-sm transition-colors hover:border-[rgba(105,137,255,0.35)]" style={{ borderColor: 'rgba(141,144,161,0.12)', background: 'var(--surface-container-lowest)' }}>
                                <p className="font-semibold">{source.title}</p>
                                <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>{source.source}</p>
                              </a>
                            ))}
                          </div>
                        ) : (
                          <p className="text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                            No citations were attached to this investigation yet.
                          </p>
                        )}
                      </Panel>
                    </section>
                  )}

                  {activeSection === 'tools' && (
                    <>
                      <Panel title="Optional Monitoring and Route Tools" icon={<FaBolt size={14} />}>
                        <div className="rounded-xl border p-4" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}>
                          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                            Linked investigation tools
                          </p>
                          <p className="mt-2 text-base font-semibold">{executionPlan.stance}</p>
                          <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                            This lane keeps the optional support tools together: external routes, watch setup, and the copyable notes. The main investigation surface stays evidence-first above.
                          </p>
                        </div>
                        <div className="mt-5 flex flex-wrap gap-3">
                          {executionPlan.primaryActions.map((action, index) => (
                            <ActionLinkButton
                              key={`${action.label}-${action.href}`}
                              action={action}
                              variant={index === 0 ? 'hero' : 'secondary'}
                              caption={index === 0 ? 'Primary route' : action.label.toLowerCase().includes('alert') ? 'Monitoring' : 'Linked route'}
                            />
                          ))}
                        </div>
                        <div className="mt-3 flex flex-wrap gap-2">
                          {executionPlan.supportingActions.map((action) => (
                            <ActionLinkButton key={`${action.label}-${action.href}`} action={action} variant="ghost" />
                          ))}
                        </div>
                      </Panel>
                      <div className="mt-6">
                        <DeepResearchPanels
                          tokenAddress={report.internal.token.contract_address}
                          preview={deepResearchPreview}
                          status={deepResearchStatus}
                          report={deepResearchReport}
                          entitlementToken={deepResearchEntitlement}
                          loading={deepResearchLoading}
                          error={deepResearchError}
                        />
                      </div>
                    </>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      </main>
    </>
  );
}

function CoverageCard({ title, text }: { title: string; text: string }) {
  return (
    <div
      className="rounded-xl border p-4"
      style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}
    >
      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {title}
      </p>
      <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
        {text}
      </p>
    </div>
  );
}

function RunConsoleChip({
  label,
  value,
  note,
}: {
  label: string;
  value: string;
  note: string;
}) {
  return (
    <div
      className="rounded-xl border p-4"
      style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}
    >
      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mt-2 text-sm font-semibold">{value}</p>
      <p className="mt-2 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
        {note}
      </p>
    </div>
  );
}

function TimelineEventRow({
  event,
}: {
  event: InvestigationRunDetailResponse['timeline'][number];
}) {
  return (
    <div
      data-testid="mia-run-console-timeline-event"
      className="rounded-xl border p-4"
      style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}
    >
      <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-semibold">{event.label}</p>
            {event.signal_tag && <SignalBadge value={event.signal_tag} testId="mia-run-console-timeline-signal-badge" />}
          </div>
          <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
            {event.detail}
          </p>
          {(event.reason || event.evidence_delta) && (
            <div className="mt-3 grid gap-3 md:grid-cols-2">
              <RunConsoleChip
                label="Reason"
                value={event.reason ?? 'n/a'}
                note="The run journal stores why this transition happened."
              />
              <RunConsoleChip
                label="Evidence delta"
                value={event.evidence_delta ?? 'n/a'}
                note="The run journal stores what materially changed at this step."
              />
            </div>
          )}
        </div>
        <span
          className="rounded-full border px-3 py-1 text-[10px] font-bold uppercase tracking-[0.16em]"
          style={{ borderColor: 'rgba(105,137,255,0.2)', color: 'var(--primary)', background: 'rgba(105,137,255,0.08)' }}
        >
          {formatDate(event.at)}
        </span>
      </div>
    </div>
  );
}

function SignalBadge({ value, testId }: { value: string; testId?: string }) {
  return (
    <span
      data-testid={testId}
      className="rounded-full border px-3 py-1 text-[10px] font-bold uppercase tracking-[0.16em]"
      style={{ borderColor: 'rgba(105,137,255,0.18)', color: 'var(--primary)', background: 'rgba(105,137,255,0.08)' }}
    >
      {signalTagLabel(value)}
    </span>
  );
}

function PrimaryActionTile({
  href,
  label,
  caption,
  tone,
  testId,
}: {
  href: string;
  label: string;
  caption: string;
  tone: 'primary' | 'safe' | 'neutral';
  testId?: string;
}) {
  const styles =
    tone === 'primary'
      ? {
          background: 'rgba(105,137,255,0.14)',
          borderColor: 'rgba(105,137,255,0.22)',
          color: 'var(--primary)',
        }
      : tone === 'safe'
        ? {
            background: 'rgba(0,255,163,0.08)',
            borderColor: 'rgba(0,255,163,0.18)',
            color: 'var(--secondary-container)',
          }
        : {
            background: 'var(--surface-container-lowest)',
            borderColor: 'rgba(141,144,161,0.12)',
            color: 'var(--on-surface)',
          };

  return (
    <Link
      href={href}
      data-testid={testId}
      className="rounded-2xl border px-4 py-4 text-left transition-colors hover:border-[rgba(105,137,255,0.35)]"
      style={styles}
    >
      <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
        {caption}
      </p>
      <p className="mt-2 text-base font-semibold">{label}</p>
    </Link>
  );
}

function ScoreOrb({
  score,
  tone,
  label,
}: {
  score: number;
  tone: 'safe' | 'primary' | 'warn' | 'danger';
  label?: string;
}) {
  const styles = toneStyles(tone);
  return (
    <div
      className="relative flex h-[92px] w-[92px] items-center justify-center rounded-full border"
      style={{
        background: `conic-gradient(${styles.color} ${clampPercent(score)}%, rgba(255,255,255,0.08) ${clampPercent(score)}% 100%)`,
        borderColor: styles.border,
      }}
    >
      <div
        className="flex h-[72px] w-[72px] flex-col items-center justify-center rounded-full border"
        style={{ background: 'rgba(10,14,24,0.96)', borderColor: 'rgba(255,255,255,0.06)' }}
      >
        <span className="font-headline text-2xl font-extrabold tracking-tight">{Math.round(score)}</span>
        <span className="text-[9px] font-bold uppercase tracking-[0.18em]" style={{ color: 'var(--outline)' }}>
          {label ?? 'Score'}
        </span>
      </div>
    </div>
  );
}

function EvidenceSignalRow({
  label,
  value,
  note,
  meter,
  tone,
}: {
  label: string;
  value: string;
  note: string;
  meter: number;
  tone: 'safe' | 'primary' | 'warn' | 'danger';
}) {
  const styles = toneStyles(tone);
  return (
    <div className="space-y-2">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <p className="text-[11px] font-bold uppercase tracking-[0.16em]" style={{ color: 'var(--outline)' }}>
            {label}
          </p>
          <p className="mt-1 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
            {note}
          </p>
        </div>
        <span className="shrink-0 text-sm font-semibold" style={{ color: styles.color }}>
          {value}
        </span>
      </div>
      <div className="h-2 rounded-full" style={{ background: 'rgba(255,255,255,0.06)' }}>
        <div className="h-2 rounded-full transition-all" style={{ width: `${meter}%`, background: styles.color }} />
      </div>
    </div>
  );
}

function WorkspaceSectionButton({
  section,
  label,
  description,
  icon,
  active,
  onClick,
}: {
  section: MiaWorkspaceSection;
  label: string;
  description: string;
  icon: React.ReactNode;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      data-testid={`mia-workspace-tab-${section}`}
      aria-pressed={active}
      onClick={onClick}
      className="group w-[16rem] rounded-[22px] border px-4 py-4 text-left shadow-[0_16px_36px_rgba(8,12,24,0.18)] transition-colors hover:border-[rgba(105,137,255,0.35)] md:w-auto"
      style={
        active
          ? {
              background: 'rgba(105,137,255,0.12)',
              borderColor: 'rgba(105,137,255,0.24)',
              color: 'var(--on-surface)',
            }
          : {
              background: 'var(--surface-container-lowest)',
              borderColor: 'rgba(141,144,161,0.12)',
              color: 'var(--on-surface)',
            }
      }
    >
      <div className="flex items-start justify-between gap-3">
        <div
          className="inline-flex h-10 w-10 items-center justify-center rounded-xl border"
          style={{
            color: active ? 'var(--primary)' : 'var(--on-surface-variant)',
            background: active ? 'rgba(105,137,255,0.12)' : 'rgba(255,255,255,0.03)',
            borderColor: active ? 'rgba(105,137,255,0.2)' : 'rgba(141,144,161,0.12)',
          }}
        >
          {icon}
        </div>
        <span
          className="rounded-full border px-2.5 py-1 text-[10px] font-bold uppercase tracking-[0.16em]"
          style={{
            color: active ? 'var(--primary)' : 'var(--outline)',
            borderColor: active ? 'rgba(105,137,255,0.2)' : 'rgba(141,144,161,0.12)',
            background: active ? 'rgba(105,137,255,0.08)' : 'rgba(255,255,255,0.02)',
          }}
        >
          {active ? 'Open now' : 'Click to open'}
        </span>
      </div>
      <p className="mt-4 font-semibold">{label}</p>
      <p className="mt-2 text-sm leading-6" style={{ color: 'var(--on-surface-variant)' }}>
        {description}
      </p>
    </button>
  );
}

function LayerAccordion({
  layer,
  open,
  readMode,
  onToggle,
}: {
  layer: MiaLayerCard;
  open: boolean;
  readMode: MiaReadMode;
  onToggle: () => void;
}) {
  const styles = toneStyles(layer.tone);

  return (
    <div className="rounded-2xl border" style={{ background: 'var(--surface-container-lowest)', borderColor: styles.border }}>
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-start justify-between gap-4 px-4 py-4 text-left"
      >
        <div>
          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: styles.color }}>
            {layer.kicker}
          </p>
          <p className="mt-1 font-semibold">{layer.title}</p>
          <p className="mt-2 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
            {layer.summary}
          </p>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-2">
          <span className="rounded-full px-2.5 py-1 text-[10px] font-bold uppercase tracking-[0.16em]" style={{ color: styles.color, background: styles.background }}>
            {Math.round(layer.meter)}/100
          </span>
          <span className="text-xs font-semibold" style={{ color: 'var(--on-surface-variant)' }}>
            {open ? 'Hide' : 'Show'}
          </span>
        </div>
      </button>

      {open && (
        <div className="border-t px-4 py-4" style={{ borderColor: 'rgba(141,144,161,0.1)' }}>
          {readMode === 'human' ? (
            <p className="text-sm leading-7">{layer.humanText}</p>
          ) : (
            <div className="space-y-2">
              {layer.analystPoints.map((point) => (
                <div
                  key={`${layer.id}-${point}`}
                  className="rounded-lg px-3 py-2 text-sm"
                  style={{ background: styles.background, color: styles.color }}
                >
                  {point}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function Panel({ title, icon, children }: { title: string; icon: React.ReactNode; children: React.ReactNode }) {
  return (
    <section
      className="rounded-2xl border p-4 shadow-[0_24px_60px_rgba(8,12,24,0.24)] md:p-6"
      style={{ background: 'linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01)), var(--surface-container-low)', borderColor: 'rgba(148,160,194,0.16)' }}
    >
      <div className="mb-4 flex items-center gap-2">
        <span style={{ color: 'var(--primary)' }}>{icon}</span>
        <h2 className="font-headline text-lg font-bold tracking-tight">{title}</h2>
      </div>
      {children}
    </section>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b pb-3 text-sm last:border-none last:pb-0" style={{ borderColor: 'rgba(141,144,161,0.08)' }}>
      <span style={{ color: 'var(--on-surface-variant)' }}>{label}</span>
      <span className="text-right font-medium">{value}</span>
    </div>
  );
}

function ListBlock({
  title,
  items,
  empty,
  tone = 'safe',
  compact = false,
}: {
  title: string;
  items: string[];
  empty: string;
  tone?: 'safe' | 'danger' | 'primary';
  compact?: boolean;
}) {
  const styles =
    tone === 'danger'
      ? { color: 'var(--danger)', background: 'rgba(255,107,107,0.08)' }
      : tone === 'primary'
        ? { color: 'var(--primary)', background: 'rgba(105,137,255,0.08)' }
        : { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' };

  return (
    <div>
      <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {title}
      </p>
      {items.length > 0 ? (
        <div className={`space-y-2 ${compact ? 'text-xs' : 'text-sm'}`}>
          {items.map((item) => (
            <div key={item} className="rounded-lg px-3 py-2" style={styles}>
              {item}
            </div>
          ))}
        </div>
      ) : (
        <p className={compact ? 'text-xs' : 'text-sm'} style={{ color: 'var(--on-surface-variant)' }}>
          {empty}
        </p>
      )}
    </div>
  );
}

function NarrativeBlock({ title, text }: { title: string; text: string }) {
  return (
    <div className="mt-4 rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(141,144,161,0.12)' }}>
      <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {title}
      </p>
      <p className="text-sm leading-7">{text}</p>
    </div>
  );
}

function StatusPill({ label, tone }: { label: string; tone: 'safe' | 'warn' | 'primary' }) {
  const style =
    tone === 'safe'
      ? { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }
      : tone === 'primary'
        ? { color: 'var(--primary)', background: 'rgba(105,137,255,0.08)' }
        : { color: 'var(--warning)', background: 'rgba(255,186,73,0.12)' };

  return (
    <span className="rounded-full px-3 py-1" style={style}>
      {label}
    </span>
  );
}

function ActionPlanCard({ title, text }: { title: string; text: string }) {
  return (
    <div className="rounded-xl border p-4" style={{ background: 'rgba(255,255,255,0.025)', borderColor: 'rgba(148,160,194,0.14)' }}>
      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {title}
      </p>
      <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
        {text}
      </p>
    </div>
  );
}

function InfoPill({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}>
      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mt-2 text-sm font-semibold">{value}</p>
      <p className="mt-2 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
        {note}
      </p>
    </div>
  );
}

function ActionLinkButton({
  action,
  variant,
  caption,
}: {
  action: { label: string; href: string; external?: boolean };
  variant: 'hero' | 'secondary' | 'ghost';
  caption?: string;
}) {
  const className =
    variant === 'hero'
      ? 'min-w-[16rem] rounded-2xl border px-4 py-4 text-left shadow-[0_20px_40px_rgba(79,110,255,0.22)] transition-colors'
      : 'rounded-xl border px-4 py-3 text-left transition-colors';
  const style =
    variant === 'hero'
      ? {
          background: 'linear-gradient(135deg, rgba(111,141,255,1), rgba(124,147,255,0.88))',
          color: 'var(--on-primary-container)',
          borderColor: 'rgba(111,141,255,0.28)',
        }
      : variant === 'secondary'
        ? {
            background: 'rgba(111,141,255,0.12)',
            color: 'var(--primary)',
            borderColor: 'rgba(111,141,255,0.2)',
          }
        : {
            background: 'rgba(255,255,255,0.025)',
            color: 'var(--on-surface)',
            borderColor: 'rgba(148,160,194,0.12)',
          };

  const body = (
    <div>
      {caption && (
        <p className="text-[10px] font-bold uppercase tracking-[0.18em]" style={{ opacity: 0.72 }}>
          {caption}
        </p>
      )}
      <p className={`font-semibold ${variant === 'hero' ? 'mt-2 text-base' : 'text-sm'}`}>{action.label}</p>
      {variant === 'hero' && (
        <p className="mt-2 text-xs" style={{ opacity: 0.76 }}>
          Open the main linked route from this investigation.
        </p>
      )}
    </div>
  );

  if (action.external) {
    return (
      <a href={action.href} target="_blank" rel="noreferrer" className={className} style={style}>
        {body}
      </a>
    );
  }

  return (
    <Link href={action.href} className={className} style={style}>
      {body}
    </Link>
  );
}

function OutcomeRow({ outcome }: { outcome: MiaOutcomeCard }) {
  const tone =
    outcome.label === 'Validated'
      ? { color: 'var(--secondary-container)', background: 'rgba(0,255,163,0.08)' }
      : outcome.label === 'Mixed'
        ? { color: 'var(--warning)', background: 'rgba(255,186,73,0.12)' }
        : { color: 'var(--danger)', background: 'rgba(255,107,107,0.08)' };

  return (
    <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}>
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="font-semibold">{shortAddress(outcome.tokenAddress, 8, 4)}</p>
          <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
            Alpha score {outcome.score.toFixed(1)} on {outcome.baselineVolume.toFixed(2)} BNB baseline flow.
          </p>
        </div>
        <span className="rounded-full px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em]" style={tone}>
          {outcome.label}
        </span>
      </div>
      <p className="mt-3 text-sm" style={{ color: tone.color }}>
        6H realized move: {formatOutcomeMove(outcome.actualMovePct)}
      </p>
    </div>
  );
}
