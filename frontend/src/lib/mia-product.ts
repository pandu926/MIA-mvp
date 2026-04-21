import type {
  AlphaBacktestResponse,
  AlphaBacktestRowResponse,
  InvestigationResponse,
  MlAlphaEvalResponse,
} from './types';

export interface MiaActionLink {
  label: string;
  href: string;
  external?: boolean;
}

export type MiaAlertMode = 'avoid' | 'watch' | 'speculative' | 'conviction' | 'exit';

export interface MiaAlertPreset {
  mode: MiaAlertMode;
  label: string;
  thresholdBnb: number;
  alphaDigestEnabled: boolean;
  rationale: string;
}

export interface MiaExecutionPlan {
  stance: string;
  sizing: string;
  entryPlan: string;
  exitPlan: string;
  alertPlan: string;
  reasons: string[];
  primaryActions: MiaActionLink[];
  supportingActions: MiaActionLink[];
}

export interface MiaProofSnapshot {
  headline: string;
  support: string;
  badge: string;
}

export interface MiaActionBrief {
  tokenAddress: string;
  tokenLabel: string;
  headline: string;
  summary: string;
  checklist: string[];
  clipboardText: string;
  alertPreset: MiaAlertPreset;
}

export interface MiaOutcomeCard {
  tokenAddress: string;
  score: number;
  actualMovePct: number;
  baselineVolume: number;
  label: 'Validated' | 'Mixed' | 'Failed';
}

function toUpperVerdict(value: string | null | undefined) {
  return (value ?? 'WATCH').trim().toUpperCase();
}

function round(value: number, digits = 1) {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

function formatMove(value: number) {
  return `${value >= 0 ? '+' : ''}${round(value, 1).toFixed(1)}%`;
}

function fourMemeTokenUrl(address: string) {
  return `https://four.meme/token/${address}`;
}

function pancakeSwapUrl(address: string) {
  return `https://pancakeswap.finance/swap?chain=bsc&outputCurrency=${address}`;
}

function shortAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}

function alertModeForVerdict(verdict: string): MiaAlertMode {
  if (verdict === 'AVOID') return 'avoid';
  if (verdict === 'WATCH') return 'watch';
  if (verdict === 'HIGH CONVICTION') return 'conviction';
  return 'speculative';
}

export function buildAlertPresetFromMode(mode: string | null | undefined): MiaAlertPreset {
  switch ((mode ?? '').trim().toLowerCase()) {
    case 'avoid':
      return {
        mode: 'avoid',
        label: 'Capital protection',
        thresholdBnb: 0.9,
        alphaDigestEnabled: false,
        rationale: 'Keep the alert layer focused on major structure changes while avoiding re-entry noise.',
      };
    case 'watch':
      return {
        mode: 'watch',
        label: 'Watch-only escalation',
        thresholdBnb: 1.0,
        alphaDigestEnabled: true,
        rationale: 'Use a higher threshold so only stronger whale behavior upgrades the setup.',
      };
    case 'conviction':
      return {
        mode: 'conviction',
        label: 'High-conviction follow-through',
        thresholdBnb: 0.8,
        alphaDigestEnabled: true,
        rationale: 'Keep alpha digest on and watch for meaningful whale reversals while the position is live.',
      };
    case 'exit':
      return {
        mode: 'exit',
        label: 'Guarded exit discipline',
        thresholdBnb: 0.5,
        alphaDigestEnabled: false,
        rationale: 'This preset prioritizes fast whale escalation over additional signal chatter after entry.',
      };
    case 'speculative':
    default:
      return {
        mode: 'speculative',
        label: 'Speculative monitoring',
        thresholdBnb: 0.6,
        alphaDigestEnabled: true,
        rationale: 'Smaller positions still need quick escalation, but alpha digest remains useful for confirmation.',
      };
  }
}

function actionCenterUrl(address: string, preset: MiaAlertPreset) {
  const params = new URLSearchParams({
    token: address,
    mode: preset.mode,
    threshold: preset.thresholdBnb.toFixed(2),
    digest: preset.alphaDigestEnabled ? '1' : '0',
  });
  return `/mia/watchlist?${params.toString()}`;
}

function convictionSignals(report: InvestigationResponse) {
  const risk = report.internal.risk?.risk_category?.toLowerCase() ?? 'unknown';
  const alphaRank = report.internal.alpha_context?.rank ?? null;
  const verifiedSource = report.contract_intelligence.source_verified;
  const criticalWhales = report.internal.whale_activity_24h.critical_alerts;
  const buyPressure = report.internal.token.buy_count - report.internal.token.sell_count;
  const deployerTrust =
    report.internal.deployer_memory?.trust_grade ?? report.internal.deployer?.trust_grade ?? null;
  const operatorConfidence = report.internal.operator_family?.confidence ?? 'low';
  const operatorSafetyScore = report.internal.operator_family?.safety_score ?? 50;

  return {
    risk,
    alphaRank,
    verifiedSource,
    criticalWhales,
    buyPressure,
    deployerTrust,
    operatorConfidence,
    operatorSafetyScore,
  };
}

function primaryScorecard(report: InvestigationResponse) {
  const internal = report.internal as InvestigationResponse['internal'] & {
    agent_scorecard?: {
      score?: number;
      label?: string;
      confidence_label?: string;
      primary_reason?: string;
      primary_risk?: string;
    } | null;
  };

  if (internal.agent_scorecard) {
    return {
      score: internal.agent_scorecard.score ?? 50,
      verdict: internal.agent_scorecard.label ?? report.analysis.label ?? report.analysis.verdict,
      confidence_label:
        internal.agent_scorecard.confidence_label ?? report.analysis.confidence ?? 'medium',
      primary_reason:
        internal.agent_scorecard.primary_reason ??
        report.analysis.primary_reason ??
        report.analysis.executive_summary,
      primary_risk:
        internal.agent_scorecard.primary_risk ??
        report.analysis.primary_risk ??
        report.analysis.risks[0] ??
        'The dominant risk is not isolated yet.',
      };
  }

  return {
    score: report.analysis.score,
    verdict: report.analysis.label ?? report.analysis.verdict ?? 'WATCH',
    confidence_label: report.analysis.confidence ?? 'medium',
    primary_reason: report.analysis.primary_reason ?? report.analysis.executive_summary,
    primary_risk:
      report.analysis.primary_risk ??
      report.analysis.risks[0] ??
      'The dominant risk is not isolated yet.',
  };
}

export function buildMiaExecutionPlan(report: InvestigationResponse): MiaExecutionPlan {
  const scorecard = primaryScorecard(report);
  const verdict = toUpperVerdict(scorecard.verdict || report.analysis.verdict);
  const alertPreset = buildAlertPresetFromMode(alertModeForVerdict(verdict));
  const {
    risk,
    alphaRank,
    verifiedSource,
    criticalWhales,
    buyPressure,
    deployerTrust,
    operatorConfidence,
    operatorSafetyScore,
  } =
    convictionSignals(report);

  const reasons = [
    scorecard.score !== null
      ? `AI investigation score is ${scorecard.score}/100 with ${scorecard.confidence_label.toLowerCase()} confidence.`
      : 'AI scoring is still locked because activity has not cleared the minimum transaction gate yet.',
    scorecard.primary_reason,
    alphaRank ? `Current alpha rank is #${alphaRank}.` : 'No current alpha rank is attached to this token.',
    verifiedSource ? 'Contract source is verified.' : 'Contract source is not verified yet.',
    criticalWhales > 0
      ? `${criticalWhales} critical whale alert(s) hit the tape in the last 24 hours.`
      : 'No critical whale alert has been recorded in the last 24 hours.',
    buyPressure >= 0
      ? `Net flow is buy-led by ${buyPressure}.`
      : `Net flow is sell-led by ${Math.abs(buyPressure)}.`,
    deployerTrust ? `Deployer trust grade is ${deployerTrust}.` : 'No deployer trust grade is available yet.',
    `Operator-family confidence is ${operatorConfidence} with safety score ${operatorSafetyScore}/100.`,
  ];

  const commonSupport: MiaActionLink[] = [
    { label: 'Open backtesting', href: '/backtesting' },
    { label: 'Open guided alerts', href: actionCenterUrl(report.token_address, alertPreset) },
  ];

  if (verdict === 'AVOID') {
    return {
      stance: 'Stand down. Do not open a fresh position from this screen.',
      sizing: '0.00x new size',
      entryPlan:
        'Keep the token on watch only if you expect a catalyst, source verification, or a cleaner alpha window. Otherwise skip it.',
      exitPlan:
        'If you already hold exposure, sell into strength and keep invalidation tight. Do not let an avoid-grade setup become a hope trade.',
      alertPlan:
        'Use Telegram only for whale and alpha escalation. The goal is to re-check the token when structure changes, not to force an entry.',
      reasons,
      primaryActions: [
        { label: 'Open sell route on Four.Meme', href: fourMemeTokenUrl(report.token_address), external: true },
        { label: 'Open PancakeSwap backup', href: pancakeSwapUrl(report.token_address), external: true },
      ],
      supportingActions: commonSupport,
    };
  }

  if (verdict === 'WATCH') {
    return {
      stance: 'Stay in observation mode until the setup earns a trade.',
      sizing: '0.10x to 0.25x probe max',
      entryPlan:
        alphaRank && alphaRank <= 10
          ? 'If you insist on participating, keep it to a probe size and wait for the next alpha window to confirm.'
          : 'Wait for cleaner confirmation: better alpha rank, source verification, or stronger buy-led flow.',
      exitPlan:
        'Take partials quickly if momentum stalls. A watch-grade setup should not survive a heavy sell-flow reversal or fresh critical whale pressure.',
      alertPlan:
        'Arm alerts before you do anything else. Let the token earn a larger size through confirmed signal improvement.',
      reasons,
      primaryActions: [
        { label: 'Open Four.Meme trade route', href: fourMemeTokenUrl(report.token_address), external: true },
        { label: 'Open guided alerts', href: actionCenterUrl(report.token_address, alertPreset) },
      ],
      supportingActions: commonSupport,
    };
  }

  if (verdict === 'HIGH CONVICTION') {
    const conservativeConditions = risk !== 'low' || !verifiedSource || criticalWhales > 0;
    return {
      stance: 'The setup is actionable, but execution still needs discipline.',
      sizing: conservativeConditions ? '0.40x to 0.65x starter size' : '0.65x to 1.00x starter size',
      entryPlan:
        alphaRank && alphaRank <= 3
          ? 'Take the trade through Four.Meme or Pancake now, then reassess after the next ranking window.'
          : 'Enter through the trade route only if you can still get a clean fill without chasing a spike.',
      exitPlan:
        risk === 'low'
          ? 'De-risk around +30% and +60%, then leave a runner only if alpha stays top-5 and whale pressure remains healthy.'
          : 'De-risk around +20% and +35%, then trail aggressively. A high-conviction call still loses status quickly in memecoin flow.',
      alertPlan:
        'Arm exit alerts immediately after entry. The purpose is to catch critical whale reversals and alpha deterioration before they cascade.',
      reasons,
      primaryActions: [
        { label: 'Open buy route on Four.Meme', href: fourMemeTokenUrl(report.token_address), external: true },
        { label: 'Open PancakeSwap backup', href: pancakeSwapUrl(report.token_address), external: true },
      ],
      supportingActions: commonSupport,
    };
  }

  return {
    stance: 'Treat this as a speculative setup, not a full-size bet.',
    sizing: '0.25x to 0.50x starter size',
    entryPlan:
      alphaRank && alphaRank <= 8
        ? 'You can take a starter through the trade route, but only with pre-defined risk and quick post-entry review.'
        : 'Wait for the next alpha pass or cleaner flow unless you are explicitly trading pure speculation.',
    exitPlan:
      'Take the first de-risk early and do not average down. If critical whale alerts appear or flow flips negative, flatten fast.',
    alertPlan:
      'Telegram routing is part of the trade plan here, not an optional extra. Speculative entries need immediate monitoring.',
    reasons,
    primaryActions: [
      { label: 'Open buy route on Four.Meme', href: fourMemeTokenUrl(report.token_address), external: true },
      { label: 'Open guided alerts', href: actionCenterUrl(report.token_address, alertPreset) },
    ],
    supportingActions: commonSupport,
  };
}

export function buildMiaActionBrief(
  report: InvestigationResponse,
  executionPlan: MiaExecutionPlan
): MiaActionBrief {
  const scorecard = primaryScorecard(report);
  const verdict = toUpperVerdict(scorecard.verdict || report.analysis.verdict);
  const alertPreset = buildAlertPresetFromMode(alertModeForVerdict(verdict));
  const tokenLabel =
    report.internal.token.symbol ??
    report.internal.token.name ??
    shortAddress(report.token_address, 8, 4);

  const headline = `${tokenLabel}: ${verdict}`;
  const summary = `${executionPlan.stance} ${executionPlan.entryPlan}`;
  const checklist = [
    `Suggested sizing: ${executionPlan.sizing}`,
    `Entry plan: ${executionPlan.entryPlan}`,
    `Exit plan: ${executionPlan.exitPlan}`,
    `Alert plan: ${executionPlan.alertPlan}`,
    `Guided preset: ${alertPreset.label} at ${alertPreset.thresholdBnb.toFixed(2)} BNB with ${alertPreset.alphaDigestEnabled ? 'alpha digest on' : 'alpha digest off'}.`,
  ];

  const clipboardText = [
    `MIA operator brief`,
    `Token: ${tokenLabel}`,
    `Contract: ${report.token_address}`,
    `Current read: ${verdict}`,
    `AI score: ${scorecard.score !== null ? `${scorecard.score}/100` : 'No score yet'}`,
    `Confidence: ${scorecard.confidence_label.toUpperCase()} (${report.analysis.conviction})`,
    `Sizing: ${executionPlan.sizing}`,
    `Entry: ${executionPlan.entryPlan}`,
    `Exit: ${executionPlan.exitPlan}`,
    `Alerts: ${executionPlan.alertPlan}`,
    `Recommended preset: ${alertPreset.label} | threshold ${alertPreset.thresholdBnb.toFixed(2)} BNB | alpha digest ${alertPreset.alphaDigestEnabled ? 'on' : 'off'}`,
  ].join('\n');

  return {
    tokenAddress: report.token_address,
    tokenLabel,
    headline,
    summary,
    checklist,
    clipboardText,
    alertPreset,
  };
}

export function buildMiaProofSnapshot(
  backtest: AlphaBacktestResponse | null,
  mlEval: MlAlphaEvalResponse | null
): MiaProofSnapshot {
  if (!backtest && !mlEval) {
    return {
      headline: 'Proof is still warming up on this deployment.',
      support: 'Run backtesting and shadow-model evaluation to replace narrative claims with measured edge.',
      badge: 'Warming up',
    };
  }

  if (mlEval && mlEval.evaluated_pairs >= 30) {
    const uplift = round(mlEval.uplift_pct_points, 2);
    const direction = uplift >= 0 ? 'outperforming' : 'underperforming';
    return {
      headline: `Shadow ML is ${direction} the legacy alpha model by ${Math.abs(uplift).toFixed(2)} percentage points.`,
      support: `${mlEval.evaluated_pairs} evaluated pairs over the last ${mlEval.hours} hours. Legacy hit rate ${mlEval.legacy_hit_rate.toFixed(2)}%, shadow ML ${mlEval.ml_hit_rate.toFixed(2)}%.`,
      badge: uplift >= 0 ? 'Measured edge' : 'Needs tuning',
    };
  }

  if (backtest) {
    return {
      headline: `Historical alpha replay is showing ${backtest.hit_rate_1h.toFixed(1)}% hit rate over ${backtest.evaluated} evaluated rows.`,
      support: `Average replay score is ${backtest.average_score_1h.toFixed(2)} at 1H and ${backtest.average_score_6h.toFixed(2)} at 6H.`,
      badge: backtest.evaluated >= 30 ? 'Replay evidence' : 'Early sample',
    };
  }

  return {
    headline: 'Replay evidence is available, but the sample size is still small.',
    support: 'Keep the claims tight until the sample clears a more reliable window.',
    badge: 'Small sample',
  };
}

export function buildRecentOutcomeCards(rows: AlphaBacktestRowResponse[], limit = 3): MiaOutcomeCard[] {
  const labelRank: Record<MiaOutcomeCard['label'], number> = {
    Validated: 0,
    Mixed: 1,
    Failed: 2,
  };

  const bestByToken = new Map<string, MiaOutcomeCard>();

  rows
    .filter((row) => row.future_volume_6h > 0 || row.future_buy_count_6h > 0 || row.future_sell_count_6h > 0)
    .forEach((row) => {
      const baseline = Math.max(row.baseline_volume_1h, 0.05);
      const actualMovePct = ((row.future_volume_6h / baseline) - 1) * 100;
      const label: MiaOutcomeCard['label'] =
        actualMovePct >= 40 ? 'Validated' : actualMovePct >= 0 ? 'Mixed' : 'Failed';

      const candidate: MiaOutcomeCard = {
        tokenAddress: row.token_address,
        score: row.alpha_score,
        actualMovePct: round(actualMovePct, 1),
        baselineVolume: row.baseline_volume_1h,
        label,
      };

      const existing = bestByToken.get(candidate.tokenAddress);
      if (!existing) {
        bestByToken.set(candidate.tokenAddress, candidate);
        return;
      }

      const candidateRank = labelRank[candidate.label];
      const existingRank = labelRank[existing.label];
      const candidateWins =
        candidateRank < existingRank ||
        (candidateRank === existingRank &&
          (candidate.score > existing.score ||
            (candidate.score === existing.score && candidate.actualMovePct > existing.actualMovePct)));

      if (candidateWins) {
        bestByToken.set(candidate.tokenAddress, candidate);
      }
    });

  return [...bestByToken.values()]
    .sort((left, right) => {
      const rankDiff = labelRank[left.label] - labelRank[right.label];
      if (rankDiff !== 0) return rankDiff;
      if (right.score !== left.score) return right.score - left.score;
      return right.actualMovePct - left.actualMovePct;
    })
    .slice(0, limit);
}

export function formatOutcomeMove(value: number) {
  return formatMove(value);
}
