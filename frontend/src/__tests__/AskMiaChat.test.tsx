import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AskMiaChat } from '@/components/mia/AskMiaChat';
import { api } from '@/lib/api';

vi.mock('@/lib/api', () => ({
  api: {
    tokens: {
      askMia: vi.fn(),
    },
    investigations: {
      getRunDetail: vi.fn(),
    },
  },
}));

describe('AskMiaChat', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.investigations.getRunDetail).mockResolvedValue({
      run: {
        run_id: 'run-123',
        token_address: '0xabc',
        trigger_type: 'manual',
        status: 'watching',
        current_stage: 'monitoring',
        source_surface: 'mia',
        current_read: 'Watch closely.',
        confidence_label: 'medium',
        investigation_score: 72,
        summary: 'Flow is still mixed.',
        status_reason: 'Monitoring reason: flow narrowed.',
        evidence_delta: 'Latest evidence delta: whale concentration increased.',
        created_at: '2026-04-19T12:00:00Z',
        updated_at: '2026-04-19T12:10:00Z',
        started_at: '2026-04-19T12:00:00Z',
        completed_at: '2026-04-19T12:05:00Z',
      },
      timeline: [],
      continuity_note: 'This run is still active.',
    });
  });

  it('submits a preset question and renders the structured answer with tool activity', async () => {
    vi.mocked(api.tokens.askMia).mockResolvedValueOnce({
      token_address: '0xabc',
      question: 'Why is this risky?',
      generated_at: '2026-04-19T12:00:00Z',
      mode: 'function_calling',
      provider: 'mia-llm',
      grounded_layers: ['verdict', 'risk', 'wallets'],
      tool_trace: ['get_token_overview', 'get_risk_snapshot'],
      run_context: {
        run_id: 'run-123',
        status: 'watching',
        current_stage: 'monitoring',
        continuity_note: 'This run is still active.',
        latest_reason: 'Monitoring reason: flow narrowed.',
        latest_evidence_delta: 'Latest evidence delta: whale concentration increased.',
        recent_events: [],
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
        why: 'Flow is narrow and builder history is mixed.',
        evidence: ['Composite risk is high.', 'Wallet concentration is elevated.'],
        next_move: 'Keep it on watch.',
      },
    });

    render(<AskMiaChat tokenAddress="0xabc" tokenLabel="TEST" runId="run-123" />);

    fireEvent.click(screen.getByRole('button', { name: 'Why is this risky?' }));

    await waitFor(() => {
      expect(api.tokens.askMia).toHaveBeenCalledWith('0xabc', {
        question: 'Why is this risky?',
        run_id: 'run-123',
      });
    });

    expect(await screen.findByText('Risk is elevated.')).toBeInTheDocument();
    expect(screen.getByText('Attached run context')).toBeInTheDocument();
    expect(screen.getByTestId('ask-mia-chat-run-aware')).toBeInTheDocument();
    expect(screen.getByText('Tool activity')).toBeInTheDocument();
    expect(screen.getByText('Token overview')).toBeInTheDocument();
    expect(screen.getByText('Risk snapshot')).toBeInTheDocument();
    expect(screen.getByText('Keep it on watch.')).toBeInTheDocument();
  });

  it('shows a safe error message when the request fails', async () => {
    vi.mocked(api.tokens.askMia).mockRejectedValueOnce(new Error('API error: 500 Internal Server Error'));

    render(<AskMiaChat tokenAddress="0xabc" tokenLabel="TEST" runId="run-123" />);

    fireEvent.change(screen.getByPlaceholderText('Ask something specific about this launch...'), {
      target: { value: 'What should I watch in the next hour?' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Send to MIA' }));

    expect(
      await screen.findByText('MIA could not answer right now. Try again in a few seconds.')
    ).toBeInTheDocument();
  });
});
