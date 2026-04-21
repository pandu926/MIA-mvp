import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api, type HealthResponse } from '@/lib/api';
import type {
  AskMiaResponse,
  DeepResearchRunResponse,
  DeepResearchRunTraceResponse,
  TokenListResponse,
} from '@/lib/types';

const MOCK_HEALTH: HealthResponse = {
  status: 'ok',
  db: 'connected',
  redis: 'connected',
  indexer: { status: 'running', last_block: 12345678 },
};

const MOCK_TOKEN_LIST: TokenListResponse = {
  data: [
    {
      contract_address: '0xabc',
      name: 'TestToken',
      symbol: 'TEST',
      deployer_address: '0xdeployer',
      deployed_at: '2026-04-10T10:00:00Z',
      block_number: 100,
      buy_count: 10,
      sell_count: 5,
      volume_bnb: 2.5,
      composite_score: 30,
      risk_category: 'low',
    },
  ],
  total: 1,
  limit: 20,
  offset: 0,
};

const MOCK_ASK_MIA: AskMiaResponse = {
  token_address: '0xabc',
  question: 'Why is this risky?',
  generated_at: '2026-04-19T12:00:00Z',
  mode: 'function_calling',
  provider: 'mia-llm',
  grounded_layers: ['verdict', 'risk'],
  tool_trace: ['get_token_overview', 'get_risk_snapshot'],
  run_context: {
    run_id: 'run-123',
    status: 'watching',
    current_stage: 'monitoring',
    continuity_note: 'This run is still active.',
    latest_reason: 'Monitoring reason: flow narrowed.',
    latest_evidence_delta: 'Latest evidence delta: whale concentration increased.',
    recent_events: [
      {
        label: 'Run status change',
        detail: 'Monitoring reason: flow narrowed.',
        at: '2026-04-19T12:00:00Z',
      },
    ],
  },
  analysis_trace: [
    {
      tool: 'get_token_overview',
      title: 'Token overview',
      detail: 'Resolve launch identity and baseline activity.',
    },
    {
      tool: 'get_risk_snapshot',
      title: 'Risk snapshot',
      detail: 'Pull the composite risk read.',
    },
  ],
  fallback_used: false,
  answer: {
    short_answer: 'Risk is elevated.',
    why: 'Wallet concentration is still high.',
    evidence: ['Risk is elevated.', 'Wallet concentration is high.'],
    next_move: 'Keep it on watch.',
  },
};

const MOCK_DEEP_RESEARCH_RUN: DeepResearchRunResponse = {
  run_id: '0f1b9803-4f3c-42b7-8bc1-fcb5759d4d53',
  token_address: '0xabc',
  provider_path: 'MIA launch intelligence + optional narrative enrichment',
  status: 'completed',
  current_phase: 'finalize',
  budget_usage_cents: 0,
  paid_calls_count: 0,
  report_ready: true,
  error_message: null,
  created_at: '2026-04-19T10:00:00Z',
  started_at: '2026-04-19T10:00:01Z',
  completed_at: '2026-04-19T10:00:03Z',
};

const MOCK_DEEP_RESEARCH_TRACE: DeepResearchRunTraceResponse = {
  run_id: MOCK_DEEP_RESEARCH_RUN.run_id,
  token_address: '0xabc',
  provider_path: 'MIA launch intelligence + optional narrative enrichment',
  status: 'completed',
  current_phase: 'finalize',
  budget_usage_cents: 0,
  paid_calls_count: 0,
  error_message: null,
  created_at: '2026-04-19T10:00:00Z',
  started_at: '2026-04-19T10:00:01Z',
  completed_at: '2026-04-19T10:00:03Z',
  steps: [],
  tool_calls: [],
  payment_ledger: [],
};

describe('api.health', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns parsed health response on success', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_HEALTH), { status: 200 })
    );

    const result = await api.health();

    expect(result).toEqual(MOCK_HEALTH);
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/health'),
      expect.objectContaining({ cache: 'no-store' })
    );
  });

  it('throws on non-ok HTTP response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response('Service Unavailable', { status: 503, statusText: 'Service Unavailable' })
    );

    await expect(api.health()).rejects.toThrow('API error: 503 Service Unavailable');
  });

  it('throws when fetch itself fails (network error)', async () => {
    vi.mocked(fetch).mockRejectedValueOnce(new Error('Network error'));

    await expect(api.health()).rejects.toThrow('Network error');
  });
});

describe('api.tokens.list', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('calls /api/v1/tokens without query params by default', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_TOKEN_LIST), { status: 200 })
    );

    await api.tokens.list();

    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/tokens'),
      expect.objectContaining({ cache: 'no-store' })
    );
  });

  it('appends risk filter to query string', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_TOKEN_LIST), { status: 200 })
    );

    await api.tokens.list({ risk: 'high' });

    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('risk=high'),
      expect.any(Object)
    );
  });

  it('appends limit and offset to query string', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_TOKEN_LIST), { status: 200 })
    );

    await api.tokens.list({ limit: 10, offset: 20 });

    const url = vi.mocked(fetch).mock.calls[0][0] as string;
    expect(url).toContain('limit=10');
    expect(url).toContain('offset=20');
  });

  it('appends activity window params to query string', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_TOKEN_LIST), { status: 200 })
    );

    await api.tokens.list({ sort: 'activity', window_hours: 168 });

    const url = vi.mocked(fetch).mock.calls[0][0] as string;
    expect(url).toContain('sort=activity');
    expect(url).toContain('window_hours=168');
  });

  it('returns parsed token list on success', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_TOKEN_LIST), { status: 200 })
    );

    const result = await api.tokens.list();

    expect(result).toEqual(MOCK_TOKEN_LIST);
    expect(result.data).toHaveLength(1);
  });

  it('throws on HTTP error', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response('Not Found', { status: 404, statusText: 'Not Found' })
    );

    await expect(api.tokens.list()).rejects.toThrow('API error: 404 Not Found');
  });
});

describe('api.tokens.askMia', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('posts the Ask MIA payload to the token route', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_ASK_MIA), { status: 200 })
    );

    const result = await api.tokens.askMia('0xabc', { question: 'Why is this risky?', run_id: 'run-123' });

    expect(result).toEqual(MOCK_ASK_MIA);
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/tokens/0xabc/ask-mia'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ question: 'Why is this risky?', run_id: 'run-123' }),
      })
    );
  });
});

describe('api.tokens.deepResearch runs', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('posts to the run creation route with entitlement header', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_DEEP_RESEARCH_RUN), { status: 200 })
    );

    const result = await api.tokens.deepResearchCreateRun('0xabc', 'entitled-token');

    expect(result).toEqual(MOCK_DEEP_RESEARCH_RUN);
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/tokens/0xabc/deep-research/runs'),
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({ 'X-MIA-ENTITLEMENT': 'entitled-token' }),
      })
    );
  });

  it('loads a run trace from the run trace route', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      new Response(JSON.stringify(MOCK_DEEP_RESEARCH_TRACE), { status: 200 })
    );

    const result = await api.tokens.deepResearchRunTrace(
      '0xabc',
      MOCK_DEEP_RESEARCH_RUN.run_id,
      'entitled-token'
    );

    expect(result).toEqual(MOCK_DEEP_RESEARCH_TRACE);
    expect(fetch).toHaveBeenCalledWith(
      expect.stringContaining(
        `/api/v1/tokens/0xabc/deep-research/runs/${MOCK_DEEP_RESEARCH_RUN.run_id}/trace`
      ),
      expect.objectContaining({
        headers: expect.objectContaining({ 'X-MIA-ENTITLEMENT': 'entitled-token' }),
      })
    );
  });
});
