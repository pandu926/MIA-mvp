import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { NarrativeCard } from '@/components/token/NarrativeCard';
import type { WsNarrativeUpdate } from '@/lib/types';

const baseNarrative: WsNarrativeUpdate = {
  type: 'narrative_update',
  token_address: '0xABC123',
  narrative_text: 'This token shows organic trading patterns with a healthy buy/sell ratio.',
  risk_interpretation: 'Low risk profile. Deployer has a clean history.',
  consensus_status: 'agreed',
  confidence: 'high',
};

describe('NarrativeCard', () => {
  it('renders narrative text', () => {
    render(<NarrativeCard narrative={baseNarrative} />);
    expect(screen.getByText(/organic trading patterns/i)).toBeInTheDocument();
  });

  it('renders risk interpretation when present', () => {
    render(<NarrativeCard narrative={baseNarrative} />);
    expect(screen.getByText(/clean history/i)).toBeInTheDocument();
  });

  it('does not render risk section when risk_interpretation is null', () => {
    const noRisk: WsNarrativeUpdate = { ...baseNarrative, risk_interpretation: null };
    render(<NarrativeCard narrative={noRisk} />);
    expect(screen.queryByText(/risk interpretation/i)).not.toBeInTheDocument();
  });

  // ─── Consensus badge ────────────────────────────────────────────────────────

  it('shows agreed consensus badge for agreed consensus', () => {
    render(<NarrativeCard narrative={baseNarrative} />);
    const badge = screen.getByText('AI Agreed');
    expect(badge).toBeInTheDocument();
  });

  it('shows diverged consensus badge for diverged consensus', () => {
    const diverged: WsNarrativeUpdate = { ...baseNarrative, consensus_status: 'diverged' };
    render(<NarrativeCard narrative={diverged} />);
    const badge = screen.getByText('AI Diverged');
    expect(badge).toBeInTheDocument();
  });

  it('shows gray Single Model badge for single_model consensus', () => {
    const single: WsNarrativeUpdate = { ...baseNarrative, consensus_status: 'single_model' };
    render(<NarrativeCard narrative={single} />);
    const badge = screen.getByText('Single Model');
    expect(badge).toBeInTheDocument();
  });

  // ─── Confidence indicator ───────────────────────────────────────────────────

  it('shows high confidence indicator', () => {
    render(<NarrativeCard narrative={baseNarrative} />);
    expect(screen.getByText(/high/i)).toBeInTheDocument();
  });

  it('shows medium confidence indicator', () => {
    const medium: WsNarrativeUpdate = { ...baseNarrative, confidence: 'medium' };
    render(<NarrativeCard narrative={medium} />);
    expect(screen.getByText(/medium/i)).toBeInTheDocument();
  });

  it('shows low confidence indicator', () => {
    const low: WsNarrativeUpdate = {
      ...baseNarrative,
      confidence: 'low',
      risk_interpretation: null, // avoid /low/i collision with "Low risk profile"
    };
    render(<NarrativeCard narrative={low} />);
    expect(screen.getByText('Low')).toBeInTheDocument();
  });

  // ─── Edge cases ─────────────────────────────────────────────────────────────

  it('renders without crashing when all optional fields null', () => {
    const minimal: WsNarrativeUpdate = {
      type: 'narrative_update',
      token_address: '0xMIN',
      narrative_text: 'Minimal narrative.',
      risk_interpretation: null,
      consensus_status: 'single_model',
      confidence: 'low',
    };
    render(<NarrativeCard narrative={minimal} />);
    expect(screen.getByText('Minimal narrative.')).toBeInTheDocument();
  });
});
