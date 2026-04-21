import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { TokenCard } from '@/components/token/TokenCard';
import type { TokenSummary } from '@/lib/types';

const baseToken: TokenSummary = {
  contract_address: '0xabcdef1234567890abcdef1234567890abcdef12',
  name: 'PepeCoin',
  symbol: 'PEPE',
  deployer_address: '0x1111111111111111111111111111111111111111',
  deployed_at: new Date(Date.now() - 60_000 * 5).toISOString(), // 5 min ago
  block_number: 12345678,
  buy_count: 80,
  sell_count: 20,
  total_tx: 100,
  volume_bnb: 5.5,
  composite_score: 42,
  risk_category: 'medium',
  ai_scored: true,
  deep_researched: false,
};

describe('TokenCard', () => {
  it('renders the token symbol', () => {
    render(<TokenCard token={baseToken} />);
    expect(screen.getByText('PEPE')).toBeInTheDocument();
  });

  it('falls back to name when symbol is null', () => {
    const token = { ...baseToken, symbol: null, name: 'PepeCoin' };
    render(<TokenCard token={token} />);
    expect(screen.getByText('PepeCoin')).toBeInTheDocument();
  });

  it('falls back to full contract address when name and symbol are null', () => {
    const token = { ...baseToken, symbol: null, name: null };
    render(<TokenCard token={token} />);
    expect(screen.getAllByText(baseToken.contract_address).length).toBeGreaterThan(0);
  });

  it('renders buy and sell counts', () => {
    render(<TokenCard token={baseToken} />);
    expect(screen.getByText('80')).toBeInTheDocument();
    expect(screen.getByText('20')).toBeInTheDocument();
  });

  it('renders volume in BNB', () => {
    render(<TokenCard token={baseToken} />);
    expect(screen.getByText('5.500 B')).toBeInTheDocument();
  });

  it('renders sell percentage', () => {
    render(<TokenCard token={baseToken} />);
    // 20 sells out of 100 total = 20%
    expect(screen.getByText('20%')).toBeInTheDocument();
  });

  it('renders the contract address on the card', () => {
    render(<TokenCard token={baseToken} />);
    expect(screen.getByText(baseToken.contract_address)).toBeInTheDocument();
  });

  it('renders risk badge with composite score', () => {
    render(<TokenCard token={baseToken} />);
    expect(screen.getByText('Score 42')).toBeInTheDocument();
  });

  it('shows scoring state when composite_score is null', () => {
    const token = { ...baseToken, composite_score: null, risk_category: null };
    render(<TokenCard token={token} />);
    expect(screen.getByText('Scoring')).toBeInTheDocument();
  });

  it('shows 70% sell ratio when sells dominate', () => {
    const token = { ...baseToken, buy_count: 30, sell_count: 70 };
    render(<TokenCard token={token} />);
    expect(screen.getByText('70%')).toBeInTheDocument();
  });
});
