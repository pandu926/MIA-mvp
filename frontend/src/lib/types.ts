// ─── API response types matching backend REST API ─────────────────────────────

export interface TokenSummary {
  contract_address: string;
  name: string | null;
  symbol: string | null;
  deployer_address: string;
  deployed_at: string;
  block_number: number;
  buy_count: number;
  sell_count: number;
  total_tx: number;
  volume_bnb: number;
  composite_score: number | null;
  risk_category: 'low' | 'medium' | 'high' | null;
  ai_scored: boolean;
  deep_researched: boolean;
  watching_for: string;
  window_hours?: number | null;
  window_volume_bnb?: number | null;
  window_buy_count?: number | null;
  window_sell_count?: number | null;
}

export interface TokenListResponse {
  data: TokenSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface TokenDetail {
  contract_address: string;
  name: string | null;
  symbol: string | null;
  deployer_address: string;
  deployed_at: string;
  block_number: number;
  tx_hash: string;
  initial_liquidity_bnb: number | null;
  holder_count: number;
  buy_count: number;
  sell_count: number;
  volume_bnb: number;
  is_rug: boolean;
  graduated: boolean;
  honeypot_detected: boolean;
  created_at: string;
}

export interface RiskDetail {
  token_address: string;
  composite_score: number;
  risk_category: 'low' | 'medium' | 'high';
  deployer_history_score: number | null;
  liquidity_lock_score: number | null;
  wallet_concentration_score: number | null;
  buy_sell_velocity_score: number | null;
  contract_audit_score: number | null;
  social_authenticity_score: number | null;
  volume_consistency_score: number | null;
  computed_at: string;
}

export interface TransactionSummary {
  tx_hash: string;
  wallet_address: string;
  tx_type: 'buy' | 'sell';
  amount_bnb: number;
  block_number: number;
  created_at: string;
}

export interface TransactionListResponse {
  data: TransactionSummary[];
  total: number;
}

export interface DeployerResponse {
  address: string;
  total_tokens_deployed: number;
  rug_count: number;
  graduated_count: number;
  honeypot_detected: boolean;
  trust_grade: 'A' | 'B' | 'C' | 'D' | 'F';
  trust_label: string;
  first_seen_at: string | null;
  last_seen_at: string | null;
}

export interface DeployerTokenResponse {
  contract_address: string;
  name: string | null;
  symbol: string | null;
  deployed_at: string;
  buy_count: number;
  sell_count: number;
  volume_bnb: number;
  composite_score: number | null;
  risk_category: 'low' | 'medium' | 'high' | null;
}

export interface WhaleAlertResponse {
  token_address: string;
  wallet_address: string;
  tx_hash: string;
  amount_bnb: number;
  threshold_bnb: number;
  alert_level: 'watch' | 'critical';
  created_at: string;
}

export interface WhaleStreamResponse {
  data: WhaleAlertResponse[];
  total: number;
  limit: number;
  offset: number;
}

export interface WhaleNetworkNode {
  id: string;
  label: string;
  node_type: 'wallet' | 'token' | string;
  wallet_address: string | null;
  token_address: string | null;
  total_volume_bnb: number;
  tx_count: number;
  critical_count: number;
  last_seen_at: string;
}

export interface WhaleNetworkEdge {
  source: string;
  target: string;
  tx_count: number;
  total_volume_bnb: number;
  last_tx_at: string;
}

export interface WhaleNetworkResponse {
  nodes: WhaleNetworkNode[];
  edges: WhaleNetworkEdge[];
  metrics: {
    total_nodes: number;
    total_edges: number;
    total_volume_bnb: number;
    critical_edges: number;
  };
  latest_updated_at: string | null;
}

export interface WalletTokenBreakdown {
  token_address: string;
  tx_count: number;
  volume_bnb: number;
  last_seen_at: string;
}

export interface WalletIntelResponse {
  wallet_address: string;
  total_whale_txs: number;
  total_volume_bnb: number;
  watch_alerts: number;
  critical_alerts: number;
  last_seen_at: string;
  top_tokens: WalletTokenBreakdown[];
}

export interface TelegramConfigResponse {
  enabled: boolean;
  chat_id: string | null;
  threshold_bnb: number;
  alpha_digest_enabled: boolean;
  updated_at: string | null;
}

export interface AlphaRowResponse {
  window_start: string;
  window_end: string;
  rank: number;
  token_address: string;
  alpha_score: number;
  rationale: string;
}

export interface AlphaBacktestRowResponse {
  window_end: string;
  rank: number;
  token_address: string;
  alpha_score: number;
  baseline_volume_1h: number;
  future_volume_1h: number;
  future_buy_count_1h: number;
  future_sell_count_1h: number;
  score_1h: number;
  outcome_1h: 'outperform' | 'neutral' | 'underperform';
  future_volume_6h: number;
  future_buy_count_6h: number;
  future_sell_count_6h: number;
  score_6h: number;
  outcome_6h: 'outperform' | 'neutral' | 'underperform';
}

export interface AlphaBacktestResponse {
  evaluated: number;
  hit_rate_1h: number;
  hit_rate_6h: number;
  average_score_1h: number;
  average_score_6h: number;
  rows: AlphaBacktestRowResponse[];
}

export interface MlAlphaEvalResponse {
  hours: number;
  evaluated_pairs: number;
  legacy_hit_rate: number;
  ml_hit_rate: number;
  uplift_pct_points: number;
}

export interface VerdictInsightResponse {
  label: string;
  tone: 'safe' | 'primary' | 'warn' | 'danger' | string;
  detail: string;
}

export interface VerdictActionResponse {
  label: string;
  href: string;
}

export interface VerdictResponse {
  token_address: string;
  label: string;
  tone: 'safe' | 'primary' | 'warn' | 'danger' | string;
  score: number;
  confidence_label: string;
  headline: string;
  summary: string;
  evidence: string[];
  concerns: string[];
  narrative_reality: VerdictInsightResponse;
  whale_intent: VerdictInsightResponse;
  deployer_dna: VerdictInsightResponse;
  next_actions: VerdictActionResponse[];
}

export interface InvestigationDeployerTokenResponse {
  contract_address: string;
  name: string | null;
  symbol: string | null;
  deployed_at: string;
  buy_count: number;
  sell_count: number;
  volume_bnb: number;
  composite_score: number | null;
  risk_category: 'low' | 'medium' | 'high' | null;
}

export interface InvestigationTransactionResponse {
  wallet_address: string;
  tx_hash: string;
  tx_type: 'buy' | 'sell' | string;
  amount_bnb: number;
  block_number: number;
  created_at: string;
}

export interface InvestigationWhaleActivityResponse {
  watch_alerts: number;
  critical_alerts: number;
  latest_levels: string[];
}

export interface DecisionSubscoreResponse {
  id: string;
  label: string;
  score: number;
  weight_pct: number;
  summary: string;
}

export interface DecisionScorecardResponse {
  decision_score: number;
  verdict: string;
  confidence_label: string;
  primary_reason: string;
  primary_risk: string;
  subscores: DecisionSubscoreResponse[];
}

export interface AgentScorecardResponse {
  score: number;
  label: string;
  confidence_label: string;
  headline: string;
  summary: string;
  primary_reason: string;
  primary_risk: string;
  supporting_points: string[];
}

export interface WalletStructureResponse {
  summary: string;
  evidence: string[];
  active_wallet_count: number;
  participant_wallet_count: number;
  holder_count: number;
  probable_cluster_wallets: number;
  potential_cluster_wallets: number;
  repeated_wallet_count: number;
  top_flow_wallets: string[];
}

export interface DeployerMemoryLaunchResponse {
  contract_address: string;
  symbol: string | null;
  name: string | null;
  is_rug: boolean;
  graduated: boolean;
  deployed_at: string;
  buy_count: number;
  sell_count: number;
  volume_bnb: number;
}

export interface DeployerMemoryResponse {
  summary: string;
  evidence: string[];
  trust_grade: string;
  trust_label: string;
  total_launches: number;
  rug_count: number;
  graduated_count: number;
  honeypot_history: boolean;
  first_seen_at: string | null;
  last_seen_at: string | null;
  recent_launches: DeployerMemoryLaunchResponse[];
}

export interface OperatorFamilyLaunchRefResponse {
  contract_address: string;
  symbol: string | null;
  name: string | null;
  deployer_address: string;
  deployed_at: string;
  is_rug: boolean;
  graduated: boolean;
  overlap_wallets: number;
}

export interface OperatorFamilyResponse {
  confidence: string;
  summary: string;
  evidence: string[];
  safety_score: number;
  signal_score: number;
  related_launch_count: number;
  related_deployer_count: number;
  repeated_wallet_count: number;
  seller_to_new_builder_count: number;
  seller_reentry_wallet_count: number;
  probable_cluster_wallets: number;
  potential_cluster_wallets: number;
  repeated_wallets: string[];
  migrated_wallets: string[];
  related_launches: OperatorFamilyLaunchRefResponse[];
}

export interface InvestigationAlphaContextResponse {
  rank: number;
  alpha_score: number;
  rationale: string;
  window_end: string;
}

export interface InvestigationNarrativeCacheResponse {
  narrative_text: string;
  risk_interpretation: string | null;
  consensus_status: string;
  confidence: string;
  generated_at: string;
  expires_at: string;
}

export interface InvestigationDeployerResponse {
  address: string;
  total_tokens_deployed: number;
  rug_count: number;
  graduated_count: number;
  honeypot_detected: boolean;
  trust_grade: string;
  trust_label: string;
  first_seen_at: string | null;
  last_seen_at: string | null;
}

export interface InvestigationRiskResponse {
  composite_score: number;
  risk_category: string;
  deployer_history_score: number | null;
  liquidity_lock_score: number | null;
  wallet_concentration_score: number | null;
  buy_sell_velocity_score: number | null;
  contract_audit_score: number | null;
  social_authenticity_score: number | null;
  volume_consistency_score: number | null;
  computed_at: string;
}

export interface InvestigationTokenResponse {
  contract_address: string;
  name: string | null;
  symbol: string | null;
  deployer_address: string;
  deployed_at: string;
  block_number: number;
  tx_hash: string;
  initial_liquidity_bnb: number | null;
  participant_wallet_count: number;
  holder_count: number;
  buy_count: number;
  sell_count: number;
  volume_bnb: number;
  is_rug: boolean;
  graduated: boolean;
  honeypot_detected: boolean;
}

export interface InvestigationHolderResponse {
  address: string;
  quantity: string;
  quantity_raw: string;
  ownership_pct: number | null;
  is_owner: boolean;
  address_type: string | null;
  owner_label: string | null;
  entity: string | null;
  is_contract: boolean | null;
}

export interface InvestigationHolderSupplyBandResponse {
  supply: string | null;
  supply_pct: number | null;
}

export interface InvestigationHolderSupplyResponse {
  top10: InvestigationHolderSupplyBandResponse;
  top25: InvestigationHolderSupplyBandResponse;
  top50: InvestigationHolderSupplyBandResponse;
  top100: InvestigationHolderSupplyBandResponse;
}

export interface InvestigationHolderChangeWindowResponse {
  change: number | null;
  change_pct: number | null;
}

export interface InvestigationHolderChangeResponse {
  one_hour: InvestigationHolderChangeWindowResponse;
  twenty_four_hours: InvestigationHolderChangeWindowResponse;
  seven_days: InvestigationHolderChangeWindowResponse;
}

export interface InvestigationHolderDistributionResponse {
  whales: number | null;
  sharks: number | null;
  dolphins: number | null;
  fish: number | null;
  octopus: number | null;
  crabs: number | null;
  shrimps: number | null;
}

export interface InvestigationHolderAcquisitionResponse {
  swap: number | null;
  transfer: number | null;
  airdrop: number | null;
}

export interface InvestigationSourceResponse {
  title: string;
  url: string;
  source: string;
}

export interface ContractIntelligenceResponse {
  provider: string;
  available: boolean;
  source_verified: boolean;
  contract_name: string | null;
  compiler_version: string | null;
  optimization_used: boolean | null;
  optimization_runs: number | null;
  proxy: boolean | null;
  implementation: string | null;
  token_type: string | null;
  total_supply: string | null;
  total_supply_raw: string | null;
  decimals: number | null;
  indexed_holder_count: number | null;
  holder_count: number | null;
  description: string | null;
  website: string | null;
  twitter: string | null;
  telegram: string | null;
  discord: string | null;
  owner_holding_pct: number | null;
  owner_in_top_holders: boolean;
  holder_supply: InvestigationHolderSupplyResponse | null;
  holder_change: InvestigationHolderChangeResponse | null;
  holder_distribution: InvestigationHolderDistributionResponse | null;
  holders_by_acquisition: InvestigationHolderAcquisitionResponse | null;
  top_holders: InvestigationHolderResponse[];
  notes: string[];
}

export interface MarketIntelligenceResponse {
  provider: string;
  available: boolean;
  x_summary: string | null;
  web_summary: string | null;
  active_event: string | null;
  narrative_alignment: string | null;
  excitement_score: number | null;
  risk_flags: string[];
  sources: InvestigationSourceResponse[];
  raw_summary: string | null;
  notes: string[];
}

export interface InvestigationAnalysisResponse {
  provider: string;
  score: number | null;
  label: string | null;
  verdict: string;
  conviction: string;
  confidence: string;
  executive_summary: string;
  primary_reason: string;
  primary_risk: string;
  supporting_points: string[];
  thesis: string[];
  risks: string[];
  next_actions: string[];
  raw: string | null;
}

export interface InvestigationSourceStatusResponse {
  bscscan_configured: boolean;
  market_provider: string;
  notes: string[];
}

export interface InvestigationDeepResearchStateResponse {
  report_cached: boolean;
  report_generated_at: string | null;
  auto_threshold_met: boolean;
  auto_threshold_tx_count: number;
  auto_requested: boolean;
  ai_score_enabled: boolean;
  ai_score_gate_tx_count: number;
  score_enriched: boolean;
}

export interface InvestigationTripwiresResponse {
  headline: string;
  watching_for: string;
  upgrade_trigger: string;
  risk_trigger: string;
  deep_research_trigger: string;
  invalidation_trigger: string;
}

export interface InvestigationInternalResponse {
  token: InvestigationTokenResponse;
  risk: InvestigationRiskResponse | null;
  agent_scorecard: AgentScorecardResponse | null;
  deployer: InvestigationDeployerResponse | null;
  deployer_recent_tokens: InvestigationDeployerTokenResponse[];
  recent_transactions: InvestigationTransactionResponse[];
  whale_activity_24h: InvestigationWhaleActivityResponse;
  alpha_context: InvestigationAlphaContextResponse | null;
  wallet_structure: WalletStructureResponse;
  deployer_memory: DeployerMemoryResponse | null;
  operator_family: OperatorFamilyResponse;
}

export interface InvestigationResponse {
  token_address: string;
  generated_at: string;
  active_run: InvestigationRunSummary | null;
  deep_research: InvestigationDeepResearchStateResponse;
  tripwires: InvestigationTripwiresResponse;
  internal: InvestigationInternalResponse;
  contract_intelligence: ContractIntelligenceResponse;
  market_intelligence: MarketIntelligenceResponse;
  analysis: InvestigationAnalysisResponse;
  source_status: InvestigationSourceStatusResponse;
}

export interface InvestigationRunSummary {
  run_id: string;
  token_address: string;
  trigger_type: string;
  status: string;
  current_stage: string;
  source_surface: string;
  current_read: string | null;
  confidence_label: string | null;
  investigation_score: number | null;
  summary: string | null;
  signal_tag: string | null;
  status_reason: string | null;
  evidence_delta: string | null;
  created_at: string;
  updated_at: string;
  started_at: string | null;
  completed_at: string | null;
}

export interface InvestigationRunTimelineEvent {
  key: string;
  label: string;
  detail: string;
  signal_tag: string | null;
  reason: string | null;
  evidence_delta: string | null;
  at: string;
}

export interface InvestigationRunDetailResponse {
  run: InvestigationRunSummary;
  timeline: InvestigationRunTimelineEvent[];
  continuity_note: string;
}

export interface InvestigationRunListResponse {
  data: InvestigationRunSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface InvestigationWatchlistItemResponse {
  item_id: string;
  entity_kind: string;
  entity_key: string;
  label: string;
  source_run_id: string | null;
  linked_runs_count: number;
  latest_run_id: string | null;
  latest_run_status: string | null;
  latest_run_token_address: string | null;
  latest_run_updated_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface InvestigationWatchlistResponse {
  data: InvestigationWatchlistItemResponse[];
  total: number;
}

export interface CreateInvestigationWatchlistItemRequest {
  entity_kind: 'token' | 'builder';
  entity_key: string;
  label?: string;
  source_run_id?: string | null;
}

export interface InvestigationMissionResponse {
  mission_id: string;
  mission_type: string;
  status: string;
  entity_kind: string | null;
  entity_key: string | null;
  label: string;
  note: string | null;
  source_watchlist_item_id: string | null;
  source_run_id: string | null;
  linked_runs_count: number;
  latest_run_id: string | null;
  latest_run_status: string | null;
  latest_run_updated_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface InvestigationMissionListResponse {
  data: InvestigationMissionResponse[];
}

export interface CreateInvestigationMissionRequest {
  mission_type:
    | 'watch_hot_launches'
    | 'watch_builder_cluster'
    | 'watch_suspicious_recurrence'
    | 'watch_proof_qualified_launches';
  entity_kind?: 'token' | 'builder';
  entity_key?: string;
  label?: string;
  note?: string;
  source_watchlist_item_id?: string | null;
  source_run_id?: string | null;
}

export interface InvestigationOpsSummaryResponse {
  runs: {
    queued: number;
    running: number;
    watching: number;
    escalated: number;
    completed: number;
    failed: number;
    archived: number;
  };
  triggers: {
    manual: number;
    auto: number;
  };
  loop_health: {
    auto_runs_24h: number;
    retry_actions_24h: number;
    recovery_actions_24h: number;
    failure_rate_24h_pct: number;
    average_completion_minutes_24h: number | null;
  };
  watchlist_items: number;
  missions: {
    active: number;
    paused: number;
    archived: number;
  };
  auto_investigation: {
    enabled: boolean;
    paused: boolean;
    tx_threshold: number;
    cooldown_mins: number;
  };
  degradation_notes: Array<{
    code: string;
    level: string;
    message: string;
  }>;
}

export interface ArchiveStaleRunsResponse {
  archived_count: number;
  archived_run_ids: string[];
  ops_summary: InvestigationOpsSummaryResponse;
}

export interface RetryFailedRunsResponse {
  retried_count: number;
  retried_run_ids: string[];
  ops_summary: InvestigationOpsSummaryResponse;
}

export interface RecoverStaleRunningRunsResponse {
  recovered_count: number;
  recovered_run_ids: string[];
  ops_summary: InvestigationOpsSummaryResponse;
}

export interface AskMiaRequest {
  question: string;
  run_id?: string;
}

export interface AskMiaAnswerResponse {
  short_answer: string;
  why: string;
  evidence: string[];
  next_move: string;
}

export interface AskMiaTraceStepResponse {
  tool: string;
  title: string;
  detail: string;
}

export interface AskMiaRunContextEventResponse {
  label: string;
  detail: string;
  at: string;
}

export interface AskMiaRunContextResponse {
  run_id: string;
  status: string;
  current_stage: string;
  continuity_note: string;
  latest_reason: string | null;
  latest_evidence_delta: string | null;
  recent_events: AskMiaRunContextEventResponse[];
}

export interface AskMiaResponse {
  token_address: string;
  question: string;
  generated_at: string;
  mode: string;
  provider: string;
  grounded_layers: string[];
  tool_trace: string[];
  analysis_trace: AskMiaTraceStepResponse[];
  run_context: AskMiaRunContextResponse | null;
  answer: AskMiaAnswerResponse;
  fallback_used: boolean;
}

export interface IntelligenceSummaryResponse {
  total_tokens: number;
  low_risk_tokens: number;
  medium_risk_tokens: number;
  high_risk_tokens: number;
  total_whale_alerts_24h: number;
  latest_alpha_window_end: string | null;
}

export interface DeepResearchSectionResponse {
  id: string;
  title: string;
  summary: string;
  stage: string;
  source_agent?: string | null;
  confidence?: string | null;
  provider?: string | null;
  source_url?: string | null;
  observed_at?: string | null;
  fallback_note?: string | null;
  evidence?: string[];
  related_tokens?: DeepResearchLinkedTokenResponse[];
  repeated_wallets?: string[];
  details?: Record<string, unknown> | null;
}

export interface DeepResearchLinkedTokenResponse {
  contract_address: string;
  symbol: string | null;
  name: string | null;
  is_rug: boolean;
  graduated: boolean;
}

export interface DeepResearchSybilPolicyResponse {
  wording: string;
  confidence_model: string;
  promise: string;
}

export interface DeepResearchPreviewResponse {
  token_address: string;
  enabled: boolean;
  provider_path: string;
  unlock_model: string;
  unlock_cta: string;
  payment_network: string;
  price_usdc_cents: number;
  sections: DeepResearchSectionResponse[];
  sybil_policy: DeepResearchSybilPolicyResponse;
  notes: string[];
}

export interface DeepResearchStatusResponse {
  token_address: string;
  premium_state: string;
  provider_path: string;
  unlock_model: string;
  x402_enabled: boolean;
  report_cached: boolean;
  has_active_entitlement: boolean;
  entitlement_expires_at: string | null;
  native_x_api_reserved: boolean;
  notes: string[];
}

export interface DeepResearchEntitlementResponse {
  access_token: string;
  kind: string;
  expires_at: string | null;
}

export interface DeepResearchReportResponse {
  token_address: string;
  provider_path: string;
  status: string;
  executive_summary: string;
  sections: DeepResearchSectionResponse[];
  citations: Record<string, unknown>[];
  source_status: Record<string, unknown>;
  generated_at: string;
  entitlement: DeepResearchEntitlementResponse | null;
}

export type DeepResearchRunStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'failed'
  | 'skipped';

export type DeepResearchRunStage =
  | 'plan'
  | 'gather_internal'
  | 'gather_external'
  | 'synthesize'
  | 'finalize';

export interface DeepResearchRunResponse {
  run_id: string;
  token_address: string;
  provider_path: string;
  status: DeepResearchRunStatus;
  current_phase: DeepResearchRunStage;
  budget_usage_cents: number;
  paid_calls_count: number;
  report_ready: boolean;
  error_message: string | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
}

export interface DeepResearchRunStepResponse {
  id: number;
  step_key: string;
  title: string;
  status: DeepResearchRunStatus;
  agent_name: string | null;
  tool_name: string | null;
  summary: string | null;
  evidence: string[];
  cost_cents: number;
  payment_tx: string | null;
  started_at: string | null;
  completed_at: string | null;
}

export interface DeepResearchToolCallResponse {
  id: number;
  step_key: string;
  tool_name: string;
  provider: string | null;
  status: DeepResearchRunStatus;
  summary: string | null;
  evidence: string[];
  latency_ms: number | null;
  cost_cents: number;
  payment_tx: string | null;
  created_at: string;
  completed_at: string | null;
}

export interface DeepResearchPaymentLedgerResponse {
  id: number;
  tool_call_id: number;
  provider: string;
  network: string;
  asset: string;
  amount_units: string;
  amount_display: string;
  tx_hash: string | null;
  status: string;
  created_at: string;
}

export interface DeepResearchRunTraceResponse {
  run_id: string;
  token_address: string;
  provider_path: string;
  status: DeepResearchRunStatus;
  current_phase: DeepResearchRunStage;
  budget_usage_cents: number;
  paid_calls_count: number;
  error_message: string | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
  steps: DeepResearchRunStepResponse[];
  tool_calls: DeepResearchToolCallResponse[];
  payment_ledger: DeepResearchPaymentLedgerResponse[];
}

export interface X402VerifyResponse {
  enabled: boolean;
  accepted: boolean;
  provider: string;
  network: string;
  scheme: string;
  facilitator_url: string;
  price_usdc_cents: number;
  status: string;
  message: string;
}

// ─── Phase 3: AI Narratives ───────────────────────────────────────────────────

export interface NarrativeResponse {
  token_address: string;
  narrative_text: string;
  risk_interpretation: string | null;
  /** "agreed" | "diverged" | "single_model" */
  consensus_status: 'agreed' | 'diverged' | 'single_model';
  /** "high" | "medium" | "low" */
  confidence: 'high' | 'medium' | 'low';
  generated_at: string;
  expires_at: string;
}

// ─── WebSocket message types ──────────────────────────────────────────────────

export interface WsTokenUpdate {
  type: 'token_update';
  token_address: string;
  name: string | null;
  symbol: string | null;
  deployer_address: string;
  buy_count: number;
  sell_count: number;
  volume_bnb: number;
  composite_score: number | null;
  risk_category: 'low' | 'medium' | 'high' | null;
  deployed_at: string;
}

export interface WsNarrativeUpdate {
  type: 'narrative_update';
  token_address: string;
  narrative_text: string;
  risk_interpretation: string | null;
  consensus_status: 'agreed' | 'diverged' | 'single_model';
  confidence: 'high' | 'medium' | 'low';
}

export interface WsPing {
  type: 'ping';
}

export interface WsPong {
  type: 'pong';
}

export type WsMessage = WsTokenUpdate | WsNarrativeUpdate | WsPing | WsPong;

// ─── Utility types ────────────────────────────────────────────────────────────

export type RiskCategory = 'low' | 'medium' | 'high';
export type TrustGrade = 'A' | 'B' | 'C' | 'D' | 'F';
export type ConsensusStatus = 'agreed' | 'diverged' | 'single_model';
export type Confidence = 'high' | 'medium' | 'low';
export type WsConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting';
