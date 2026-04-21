import type {
  AlphaBacktestResponse,
  AlphaRowResponse,
  DeployerTokenResponse,
  DeployerResponse,
  DeepResearchPreviewResponse,
  DeepResearchRunResponse,
  DeepResearchRunTraceResponse,
  DeepResearchReportResponse,
  DeepResearchStatusResponse,
  AskMiaRequest,
  AskMiaResponse,
  IntelligenceSummaryResponse,
  InvestigationResponse,
  InvestigationRunDetailResponse,
  InvestigationRunListResponse,
  InvestigationRunSummary,
  InvestigationWatchlistResponse,
  InvestigationWatchlistItemResponse,
  CreateInvestigationWatchlistItemRequest,
  InvestigationMissionListResponse,
  InvestigationMissionResponse,
  CreateInvestigationMissionRequest,
  InvestigationOpsSummaryResponse,
  ArchiveStaleRunsResponse,
  RetryFailedRunsResponse,
  RecoverStaleRunningRunsResponse,
  MlAlphaEvalResponse,
  NarrativeResponse,
  RiskDetail,
  TelegramConfigResponse,
  TokenDetail,
  TokenListResponse,
  TransactionListResponse,
  VerdictResponse,
  WhaleAlertResponse,
  WhaleNetworkResponse,
  WhaleStreamResponse,
  WalletIntelResponse,
  X402VerifyResponse,
} from './types';

const API_BASE_URL =
  typeof window === 'undefined'
    ? process.env.INTERNAL_API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? 'http://backend:8080'
    : process.env.NEXT_PUBLIC_API_URL ?? '/api/backend';

export interface HealthResponse {
  status: string;
  db: string;
  redis: string;
  indexer: {
    status: string;
    last_block: number;
  };
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    cache: 'no-store',
    ...init,
  });
  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }
  return response.json() as Promise<T>;
}

async function fetchJsonWithHeaders<T>(path: string, init?: RequestInit): Promise<{ data: T; headers: Headers; status: number }> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    cache: 'no-store',
    ...init,
  });
  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }
  const data = (await response.json()) as T;
  return { data, headers: response.headers, status: response.status };
}

export interface TokenListParams {
  limit?: number;
  offset?: number;
  risk?: 'low' | 'medium' | 'high';
  q?: string;
  min_liquidity?: number;
  sort?: 'newest' | 'volume' | 'risk' | 'activity' | 'tx';
  window_hours?: number;
  ai_scored?: boolean;
  deep_research?: boolean;
}

export interface InvestigationRunListParams {
  limit?: number;
  offset?: number;
  status?: string;
  trigger?: string;
  token?: string;
}

export const api = {
  health: () => fetchJson<HealthResponse>('/health'),

  tokens: {
    list: (params: TokenListParams = {}) => {
      const query = new URLSearchParams();
      if (params.limit !== undefined) query.set('limit', String(params.limit));
      if (params.offset !== undefined) query.set('offset', String(params.offset));
      if (params.risk) query.set('risk', params.risk);
      if (params.q) query.set('q', params.q);
      if (params.min_liquidity !== undefined) query.set('min_liquidity', String(params.min_liquidity));
      if (params.sort) query.set('sort', params.sort);
      if (params.window_hours !== undefined) query.set('window_hours', String(params.window_hours));
      if (params.ai_scored !== undefined) query.set('ai_scored', String(params.ai_scored));
      if (params.deep_research !== undefined) query.set('deep_research', String(params.deep_research));
      const qs = query.toString();
      return fetchJson<TokenListResponse>(`/api/v1/tokens${qs ? `?${qs}` : ''}`);
    },
    get: (address: string) => fetchJson<TokenDetail>(`/api/v1/tokens/${address}`),
    risk: (address: string) => fetchJson<RiskDetail>(`/api/v1/tokens/${address}/risk`),
    transactions: (address: string) =>
      fetchJson<TransactionListResponse>(`/api/v1/tokens/${address}/transactions`),
    narrative: (address: string) =>
      fetchJson<NarrativeResponse>(`/api/v1/tokens/${address}/narrative`),
    verdict: (address: string) =>
      fetchJson<VerdictResponse>(`/api/v1/tokens/${address}/verdict`),
    investigation: (address: string) =>
      fetchJson<InvestigationResponse>(`/api/v1/tokens/${address}/investigation`),
    askMia: (address: string, payload: AskMiaRequest) =>
      fetchJson<AskMiaResponse>(`/api/v1/tokens/${address}/ask-mia`, {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
    deepResearchPreview: (address: string) =>
      fetchJson<DeepResearchPreviewResponse>(`/api/v1/tokens/${address}/deep-research/preview`, {
        method: 'POST',
      }),
    deepResearchStatus: (address: string) =>
      fetchJson<DeepResearchStatusResponse>(`/api/v1/tokens/${address}/deep-research/status`),
    deepResearchReport: (address: string, entitlementToken?: string | null) =>
      fetchJsonWithHeaders<DeepResearchReportResponse>(`/api/v1/tokens/${address}/deep-research`, {
        headers: entitlementToken ? { 'X-MIA-ENTITLEMENT': entitlementToken } : undefined,
      }),
    deepResearchCreateRun: (address: string, entitlementToken?: string | null) =>
      fetchJson<DeepResearchRunResponse>(`/api/v1/tokens/${address}/deep-research/runs`, {
        method: 'POST',
        headers: entitlementToken ? { 'X-MIA-ENTITLEMENT': entitlementToken } : undefined,
      }),
    deepResearchRun: (address: string, runId: string, entitlementToken?: string | null) =>
      fetchJson<DeepResearchRunResponse>(`/api/v1/tokens/${address}/deep-research/runs/${runId}`, {
        headers: entitlementToken ? { 'X-MIA-ENTITLEMENT': entitlementToken } : undefined,
      }),
    deepResearchRunTrace: (address: string, runId: string, entitlementToken?: string | null) =>
      fetchJson<DeepResearchRunTraceResponse>(
        `/api/v1/tokens/${address}/deep-research/runs/${runId}/trace`,
        {
          headers: entitlementToken ? { 'X-MIA-ENTITLEMENT': entitlementToken } : undefined,
        }
      ),
    deepResearchRunReport: (address: string, runId: string, entitlementToken?: string | null) =>
      fetchJsonWithHeaders<DeepResearchReportResponse>(
        `/api/v1/tokens/${address}/deep-research/runs/${runId}/report`,
        {
          headers: entitlementToken ? { 'X-MIA-ENTITLEMENT': entitlementToken } : undefined,
        }
      ),
  },

  deployer: {
    get: (address: string) => fetchJson<DeployerResponse>(`/api/v1/deployer/${address}`),
    tokens: (address: string, limit = 20) =>
      fetchJson<DeployerTokenResponse[]>(`/api/v1/deployer/${address}/tokens?limit=${limit}`),
  },

  whales: {
    list: (limit = 20) => fetchJson<WhaleAlertResponse[]>(`/api/v1/whales?limit=${limit}`),
    stream: (params: {
      limit?: number;
      offset?: number;
      min_amount?: number;
      level?: 'watch' | 'critical';
      token?: string;
    } = {}) => {
      const query = new URLSearchParams();
      if (params.limit !== undefined) query.set('limit', String(params.limit));
      if (params.offset !== undefined) query.set('offset', String(params.offset));
      if (params.min_amount !== undefined) query.set('min_amount', String(params.min_amount));
      if (params.level) query.set('level', params.level);
      if (params.token) query.set('token', params.token);
      const qs = query.toString();
      return fetchJson<WhaleStreamResponse>(`/api/v1/whales/stream${qs ? `?${qs}` : ''}`);
    },
    network: (params: {
      hours?: number;
      min_amount?: number;
      level?: 'watch' | 'critical';
    } = {}) => {
      const query = new URLSearchParams();
      if (params.hours !== undefined) query.set('hours', String(params.hours));
      if (params.min_amount !== undefined) query.set('min_amount', String(params.min_amount));
      if (params.level) query.set('level', params.level);
      const qs = query.toString();
      return fetchJson<WhaleNetworkResponse>(`/api/v1/whales/network${qs ? `?${qs}` : ''}`);
    },
  },

  alpha: {
    latest: (limit = 10) => fetchJson<AlphaRowResponse[]>(`/api/v1/alpha/latest?limit=${limit}`),
    history: (hours = 24, limit = 100) =>
      fetchJson<AlphaRowResponse[]>(`/api/v1/alpha/history?hours=${hours}&limit=${limit}`),
    backtest: (hours = 24, limit = 120) =>
      fetchJson<AlphaBacktestResponse>(`/api/v1/alpha/backtest?hours=${hours}&limit=${limit}`),
  },

  intelligence: {
    summary: () => fetchJson<IntelligenceSummaryResponse>('/api/v1/intelligence/summary'),
  },

  investigations: {
    runs: (params: InvestigationRunListParams = {}) => {
      const query = new URLSearchParams();
      if (params.limit !== undefined) query.set('limit', String(params.limit));
      if (params.offset !== undefined) query.set('offset', String(params.offset));
      if (params.status) query.set('status', params.status);
      if (params.trigger) query.set('trigger', params.trigger);
      if (params.token) query.set('token', params.token);
      const qs = query.toString();
      return fetchJson<InvestigationRunListResponse>(`/api/v1/investigations/runs${qs ? `?${qs}` : ''}`);
    },
    getRun: (runId: string) => fetchJson<InvestigationRunSummary>(`/api/v1/investigations/runs/${runId}`),
    getRunDetail: (runId: string) =>
      fetchJson<InvestigationRunDetailResponse>(`/api/v1/investigations/runs/${runId}/detail`),
    updateRunStatus: (
      runId: string,
      payload: {
        status: 'watching' | 'escalated' | 'archived';
        reason?: string;
        evidence_delta?: string;
      }
    ) =>
      fetchJson<InvestigationRunSummary>(`/api/v1/investigations/runs/${runId}/status`, {
        method: 'PATCH',
        body: JSON.stringify(payload),
      }),
    watchlist: () => fetchJson<InvestigationWatchlistResponse>('/api/v1/investigations/watchlist'),
    createWatchlistItem: (payload: CreateInvestigationWatchlistItemRequest) =>
      fetchJson<InvestigationWatchlistItemResponse>('/api/v1/investigations/watchlist', {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
    deleteWatchlistItem: (itemId: string) =>
      fetchJson<{ deleted: boolean; item_id: string }>(`/api/v1/investigations/watchlist/${itemId}`, {
        method: 'DELETE',
      }),
    missions: () => fetchJson<InvestigationMissionListResponse>('/api/v1/investigations/missions'),
    createMission: (payload: CreateInvestigationMissionRequest) =>
      fetchJson<InvestigationMissionResponse>('/api/v1/investigations/missions', {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
    updateMissionStatus: (missionId: string, status: 'active' | 'paused' | 'archived') =>
      fetchJson<InvestigationMissionResponse>(`/api/v1/investigations/missions/${missionId}`, {
        method: 'PATCH',
        body: JSON.stringify({ status }),
      }),
    opsSummary: () => fetchJson<InvestigationOpsSummaryResponse>('/api/v1/investigations/ops/summary'),
    updateOpsControl: (payload: { auto_investigation_paused: boolean }) =>
      fetchJson<InvestigationOpsSummaryResponse>('/api/v1/investigations/ops/summary', {
        method: 'PATCH',
        body: JSON.stringify(payload),
      }),
    archiveStaleRuns: (payload: { stale_after_minutes?: number }) =>
      fetchJson<ArchiveStaleRunsResponse>('/api/v1/investigations/ops/archive-stale', {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
    retryFailedRuns: () =>
      fetchJson<RetryFailedRunsResponse>('/api/v1/investigations/ops/retry-failed', {
        method: 'POST',
      }),
    recoverStaleRunningRuns: (payload: { stale_after_minutes?: number } = {}) =>
      fetchJson<RecoverStaleRunningRunsResponse>('/api/v1/investigations/ops/recover-stale-running', {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
  },

  ml: {
    alphaEval: (hours = 168) =>
      fetchJson<MlAlphaEvalResponse>(`/api/v1/ml/alpha/eval?hours=${hours}`),
  },

  wallets: {
    intel: (address: string, hours = 24) =>
      fetchJson<WalletIntelResponse>(`/api/v1/wallets/${address}/intel?hours=${hours}`),
  },

  telegram: {
    getConfig: () => fetchJson<TelegramConfigResponse>('/api/v1/telegram/config'),
    updateConfig: (payload: {
      enabled: boolean;
      chat_id: string | null;
      threshold_bnb: number;
      alpha_digest_enabled: boolean;
    }) =>
      fetchJson<TelegramConfigResponse>('/api/v1/telegram/config', {
        method: 'PUT',
        body: JSON.stringify(payload),
      }),
  },

  payments: {
    verifyX402: (payload: {
      token_address?: string | null;
      resource?: string | null;
      payment_payload?: unknown;
    }) =>
      fetchJson<X402VerifyResponse>('/api/v1/x402/verify', {
        method: 'POST',
        body: JSON.stringify(payload),
      }),
  },
};
