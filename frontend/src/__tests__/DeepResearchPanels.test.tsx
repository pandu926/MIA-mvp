import type { AnchorHTMLAttributes } from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DeepResearchPanels } from '@/components/mia/DeepResearchPanels';
import { api } from '@/lib/api';
import type {
  DeepResearchPreviewResponse,
  DeepResearchReportResponse,
  DeepResearchRunResponse,
  DeepResearchRunTraceResponse,
  DeepResearchStatusResponse,
} from '@/lib/types';

vi.mock('next/link', () => ({
  default: ({ children, href, ...props }: AnchorHTMLAttributes<HTMLAnchorElement>) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
}));

vi.mock('@/lib/api', () => ({
  api: {
    tokens: {
      deepResearchCreateRun: vi.fn(),
      deepResearchRunTrace: vi.fn(),
      deepResearchRunReport: vi.fn(),
      deepResearchRun: vi.fn(),
    },
  },
}));

const preview: DeepResearchPreviewResponse = {
  token_address: '0xabc',
  enabled: true,
  provider_path: 'MIA launch intelligence + optional narrative enrichment',
  unlock_model: 'x402',
  unlock_cta: 'Unlock Deep Research',
  payment_network: 'base',
  price_usdc_cents: 50,
  sections: [
    {
      id: 'dex-market',
      title: 'Dex market structure',
      summary: 'Attach market structure.',
      stage: 'mvp',
    },
  ],
  sybil_policy: {
    wording: 'pattern warning',
    confidence_model: 'signal-based',
    promise: 'no identity claims',
  },
  notes: [],
};

const status: DeepResearchStatusResponse = {
  token_address: '0xabc',
  premium_state: 'report_ready',
  provider_path: 'MIA launch intelligence + optional narrative enrichment',
  unlock_model: 'x402',
  x402_enabled: true,
  report_cached: true,
  has_active_entitlement: true,
  entitlement_expires_at: null,
  native_x_api_reserved: true,
  notes: [],
};

const run: DeepResearchRunResponse = {
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

const trace: DeepResearchRunTraceResponse = {
  run_id: run.run_id,
  token_address: '0xabc',
  provider_path: run.provider_path,
  status: 'completed',
  current_phase: 'finalize',
  budget_usage_cents: 0,
  paid_calls_count: 0,
  error_message: null,
  created_at: run.created_at,
  started_at: run.started_at,
  completed_at: run.completed_at,
  steps: [
    {
      id: 1,
      step_key: 'plan',
      title: 'Plan',
      status: 'completed',
      agent_name: 'PlannerAgent',
      tool_name: 'get_market_structure, get_wallet_structure',
      summary: 'Planner created a stable internal research plan.',
      evidence: ['Planned steps: 5.', 'Required tools: 4.'],
      cost_cents: 0,
      payment_tx: null,
      started_at: run.started_at,
      completed_at: run.completed_at,
    },
  ],
  tool_calls: [
    {
      id: 1,
      step_key: 'gather_internal',
      tool_name: 'get_market_structure',
      provider: 'mia_internal',
      status: 'completed',
      summary: 'Market structure attached.',
      evidence: ['Liquidity quality is stable.'],
      latency_ms: 85,
      cost_cents: 0,
      payment_tx: null,
      created_at: run.created_at,
      completed_at: run.completed_at,
    },
  ],
  payment_ledger: [
    {
      id: 1,
      tool_call_id: 1,
      provider: 'heurist_mesh_x402',
      network: 'base',
      asset: 'USDC',
      amount_units: '2000',
      amount_display: '0.002000 USDC',
      tx_hash: '0xa34b7ab7843b63decac5e3134402023d488978f17a6ac8779df51b2ccf6ce607',
      status: 'completed',
      created_at: run.created_at,
    },
  ],
};

const report: DeepResearchReportResponse = {
  token_address: '0xabc',
  provider_path: run.provider_path,
  status: 'ready',
  executive_summary: 'Premium dossier assembled successfully.',
  sections: [
    {
      id: 'wallet-structure',
      title: 'Wallet structure',
      summary: 'Wallet concentration is elevated.',
      stage: 'mvp',
      provider: 'mia_internal',
      confidence: 'high',
      evidence: ['Repeated wallets were recovered.'],
    },
  ],
  citations: [],
  source_status: {},
  generated_at: '2026-04-19T10:00:03Z',
  entitlement: null,
};

describe('DeepResearchPanels', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
  });

  it('starts a run and renders the trace workspace', async () => {
    vi.mocked(api.tokens.deepResearchCreateRun).mockResolvedValue(run);
    vi.mocked(api.tokens.deepResearchRunTrace).mockResolvedValue(trace);
    vi.mocked(api.tokens.deepResearchRunReport).mockResolvedValue({
      data: report,
      headers: new Headers(),
      status: 200,
    });

    render(
      <DeepResearchPanels
        tokenAddress="0xabc"
        preview={preview}
        status={status}
        report={null}
        entitlementToken="entitled-token"
        loading={false}
        error={null}
      />
    );

    fireEvent.click(screen.getByTestId('deep-research-start-run'));

    await waitFor(() =>
      expect(api.tokens.deepResearchCreateRun).toHaveBeenCalledWith(
        '0xabc',
        'entitled-token'
      )
    );

    await waitFor(() =>
      expect(screen.getByTestId('deep-research-run-header')).toBeInTheDocument()
    );
    expect(screen.getByTestId('deep-research-run-trace')).toHaveTextContent(
      'Planner created a stable internal research plan.'
    );
    expect(screen.getByTestId('deep-research-tool-ledger')).toHaveTextContent(
      'get market structure'
    );
    expect(screen.getByText('Premium dossier assembled successfully.')).toBeInTheDocument();
  });
});
