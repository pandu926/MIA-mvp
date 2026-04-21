import { describe, expect, it } from 'vitest';
import {
  buildAlertPresetFromMode,
  buildMiaActionBrief,
  buildMiaExecutionPlan,
  buildMiaProofSnapshot,
  buildRecentOutcomeCards,
  formatOutcomeMove,
} from '@/lib/mia-product';
import type { AlphaBacktestResponse, InvestigationResponse, MlAlphaEvalResponse } from '@/lib/types';

function makeInvestigation(overrides: Partial<InvestigationResponse> = {}): InvestigationResponse {
  return {
    token_address: '0x1234567890abcdef1234567890abcdef12345678',
    generated_at: '2026-04-17T12:00:00Z',
    active_run: null,
    deep_research: {
      report_cached: false,
      report_generated_at: null,
      auto_threshold_met: false,
      auto_threshold_tx_count: 500,
      auto_requested: false,
      ai_score_enabled: true,
      ai_score_gate_tx_count: 50,
      score_enriched: false,
    },
    internal: {
      token: {
        contract_address: '0x1234567890abcdef1234567890abcdef12345678',
        name: 'Test Token',
        symbol: 'TEST',
        deployer_address: '0xdeployer',
        deployed_at: '2026-04-17T11:00:00Z',
        block_number: 1,
        tx_hash: '0xtx',
        initial_liquidity_bnb: 1.5,
        participant_wallet_count: 200,
        holder_count: 200,
        buy_count: 50,
        sell_count: 20,
        volume_bnb: 4.5,
        is_rug: false,
        graduated: false,
        honeypot_detected: false,
      },
      risk: {
        composite_score: 28,
        risk_category: 'low',
        deployer_history_score: 10,
        liquidity_lock_score: 10,
        wallet_concentration_score: 10,
        buy_sell_velocity_score: 10,
        contract_audit_score: 10,
        social_authenticity_score: 10,
        volume_consistency_score: 10,
        computed_at: '2026-04-17T12:00:00Z',
      },
      agent_scorecard: {
        score: 88,
        label: 'HIGH CONVICTION',
        confidence_label: 'high',
        headline: 'Momentum is aligned.',
        summary: 'Summary',
        primary_reason: 'Momentum is aligned.',
        primary_risk: 'No dominant risk.',
        supporting_points: [],
      },
      deployer: {
        address: '0xdeployer',
        total_tokens_deployed: 5,
        rug_count: 0,
        graduated_count: 2,
        honeypot_detected: false,
        trust_grade: 'A',
        trust_label: 'Trusted',
        first_seen_at: null,
        last_seen_at: null,
      },
      deployer_recent_tokens: [],
      recent_transactions: [],
      whale_activity_24h: {
        watch_alerts: 2,
        critical_alerts: 0,
        latest_levels: ['watch'],
      },
      alpha_context: {
        rank: 2,
        alpha_score: 84,
        rationale: 'Strong',
        window_end: '2026-04-17T12:00:00Z',
      },
      wallet_structure: {
        summary: 'Broad wallet participation.',
        evidence: [],
        active_wallet_count: 160,
        participant_wallet_count: 200,
        holder_count: 200,
        probable_cluster_wallets: 2,
        potential_cluster_wallets: 1,
        repeated_wallet_count: 0,
        top_flow_wallets: [],
      },
      deployer_memory: {
        summary: 'Deployer has prior clean launches.',
        evidence: [],
        trust_grade: 'A',
        trust_label: 'Trusted',
        total_launches: 5,
        rug_count: 0,
        graduated_count: 2,
        honeypot_history: false,
        first_seen_at: null,
        last_seen_at: null,
        recent_launches: [],
      },
      operator_family: {
        confidence: 'low',
        summary: 'No strong overlap signal.',
        evidence: [],
        safety_score: 80,
        signal_score: 15,
        related_launch_count: 0,
        related_deployer_count: 0,
        repeated_wallet_count: 0,
        seller_to_new_builder_count: 0,
        seller_reentry_wallet_count: 0,
        probable_cluster_wallets: 2,
        potential_cluster_wallets: 1,
        repeated_wallets: [],
        migrated_wallets: [],
        related_launches: [],
      },
    },
    contract_intelligence: {
      provider: 'bscscan',
      available: true,
      source_verified: true,
      contract_name: 'Test',
      compiler_version: '0.8.0',
      optimization_used: true,
      optimization_runs: 200,
      proxy: false,
      implementation: null,
      token_type: 'ERC20',
      total_supply: '1000000',
      total_supply_raw: '1000000',
      decimals: 18,
      indexed_holder_count: 200,
      holder_count: 200,
      holder_supply: null,
      holder_change: null,
      holder_distribution: null,
      holders_by_acquisition: null,
      description: null,
      website: null,
      twitter: null,
      telegram: null,
      discord: null,
      owner_holding_pct: null,
      owner_in_top_holders: false,
      top_holders: [],
      notes: [],
    },
    market_intelligence: {
      provider: 'xai',
      available: true,
      x_summary: null,
      web_summary: null,
      active_event: 'Narrative acceleration',
      narrative_alignment: 'Aligned',
      excitement_score: 77,
      risk_flags: [],
      sources: [],
      raw_summary: null,
      notes: [],
    },
    analysis: {
      provider: 'mia',
      score: 88,
      label: 'HIGH CONVICTION',
      verdict: 'HIGH CONVICTION',
      conviction: 'Strong',
      confidence: 'high',
      executive_summary: 'Summary',
      primary_reason: 'Momentum is aligned.',
      primary_risk: 'No dominant risk.',
      supporting_points: [],
      thesis: [],
      risks: [],
      next_actions: [],
      raw: null,
    },
    source_status: {
      bscscan_configured: true,
      market_provider: 'google-news-rss',
      notes: [],
    },
    ...overrides,
  };
}

const backtest: AlphaBacktestResponse = {
  evaluated: 120,
  hit_rate_1h: 58.2,
  hit_rate_6h: 61.4,
  average_score_1h: 24.3,
  average_score_6h: 31.8,
  rows: [
    {
      window_end: '2026-04-17T10:00:00Z',
      rank: 1,
      token_address: '0xaaa',
      alpha_score: 91,
      baseline_volume_1h: 1,
      future_volume_1h: 1.8,
      future_buy_count_1h: 10,
      future_sell_count_1h: 2,
      score_1h: 70,
      outcome_1h: 'outperform',
      future_volume_6h: 2.1,
      future_buy_count_6h: 13,
      future_sell_count_6h: 4,
      score_6h: 82,
      outcome_6h: 'outperform',
    },
    {
      window_end: '2026-04-17T09:00:00Z',
      rank: 2,
      token_address: '0xbbb',
      alpha_score: 79,
      baseline_volume_1h: 1,
      future_volume_1h: 0.9,
      future_buy_count_1h: 4,
      future_sell_count_1h: 5,
      score_1h: -10,
      outcome_1h: 'neutral',
      future_volume_6h: 0.8,
      future_buy_count_6h: 8,
      future_sell_count_6h: 9,
      score_6h: -25,
      outcome_6h: 'underperform',
    },
  ],
};

describe('buildMiaExecutionPlan', () => {
  it('builds actionable trade guidance for high-conviction setups', () => {
    const plan = buildMiaExecutionPlan(makeInvestigation());

    expect(plan.sizing).toContain('0.65x');
    expect(plan.primaryActions[0]?.href).toContain('https://four.meme/token/');
    expect(plan.supportingActions.some((item) => item.href.includes('/mia/watchlist?token='))).toBe(true);
    expect(plan.supportingActions.some((item) => item.href.includes('mode=conviction'))).toBe(true);
  });

  it('blocks fresh entries for avoid verdicts', () => {
    const plan = buildMiaExecutionPlan(
      makeInvestigation({
        analysis: {
          provider: 'mia',
          score: 18,
          label: 'AVOID',
          verdict: 'AVOID',
          conviction: 'Low',
          confidence: 'high',
          executive_summary: 'Avoid this.',
          primary_reason: 'Avoid this.',
          primary_risk: 'Thin structure.',
          supporting_points: [],
          thesis: [],
          risks: [],
          next_actions: [],
          raw: null,
        },
      })
    );

    expect(plan.sizing).toBe('0.00x new size');
    expect(plan.stance).toContain('Do not open a fresh position');
  });
});

describe('buildMiaActionBrief', () => {
  it('creates a copyable operator brief with a guided preset', () => {
    const investigation = makeInvestigation();
    const plan = buildMiaExecutionPlan(investigation);
    const brief = buildMiaActionBrief(investigation, plan);

    expect(brief.headline).toContain('HIGH CONVICTION');
    expect(brief.alertPreset.mode).toBe('conviction');
    expect(brief.clipboardText).toContain('MIA operator brief');
    expect(brief.checklist.some((item) => item.includes('Guided preset'))).toBe(true);
  });
});

describe('buildAlertPresetFromMode', () => {
  it('maps exit mode to a guarded preset', () => {
    const preset = buildAlertPresetFromMode('exit');

    expect(preset.mode).toBe('exit');
    expect(preset.thresholdBnb).toBe(0.5);
    expect(preset.alphaDigestEnabled).toBe(false);
  });
});

describe('buildMiaProofSnapshot', () => {
  it('prefers ML evidence when available', () => {
    const mlEval: MlAlphaEvalResponse = {
      hours: 168,
      evaluated_pairs: 155,
      legacy_hit_rate: 7.31,
      ml_hit_rate: 9.68,
      uplift_pct_points: 2.37,
    };

    const proof = buildMiaProofSnapshot(backtest, mlEval);
    expect(proof.headline).toContain('outperforming');
    expect(proof.badge).toBe('Measured edge');
  });
});

describe('buildRecentOutcomeCards', () => {
  it('formats the strongest recent outcomes first', () => {
    const cards = buildRecentOutcomeCards(backtest.rows, 2);

    expect(cards).toHaveLength(2);
    expect(cards[0]?.tokenAddress).toBe('0xaaa');
    expect(cards[0]?.label).toBe('Validated');
    expect(formatOutcomeMove(cards[0]?.actualMovePct ?? 0)).toMatch(/^\+/);
  });

  it('prefers validated cards over failed ones and deduplicates repeated tokens', () => {
    const cards = buildRecentOutcomeCards(
      [
        {
          ...backtest.rows[1],
          token_address: '0xdup',
          alpha_score: 95,
          baseline_volume_1h: 1,
          future_volume_6h: 0.2,
        },
        {
          ...backtest.rows[0],
          token_address: '0xdup',
          alpha_score: 35,
          baseline_volume_1h: 1,
          future_volume_6h: 2.4,
        },
        {
          ...backtest.rows[1],
          token_address: '0xfail',
          alpha_score: 99,
          baseline_volume_1h: 1,
          future_volume_6h: 0.1,
        },
      ],
      3
    );

    expect(cards).toHaveLength(2);
    expect(cards[0]?.tokenAddress).toBe('0xdup');
    expect(cards[0]?.label).toBe('Validated');
    expect(cards[1]?.tokenAddress).toBe('0xfail');
    expect(cards[1]?.label).toBe('Failed');
  });
});
