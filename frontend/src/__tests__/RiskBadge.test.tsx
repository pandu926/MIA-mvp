import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { RiskBadge } from '@/components/token/RiskBadge';

describe('RiskBadge', () => {
  it('shows score and Low Risk label for low category', () => {
    render(<RiskBadge score={25} category="low" />);
    expect(screen.getByText('25 · Safe')).toBeInTheDocument();
  });

  it('shows score and Medium Risk label for medium category', () => {
    render(<RiskBadge score={50} category="medium" />);
    expect(screen.getByText('50 · Caution')).toBeInTheDocument();
  });

  it('shows score and High Risk label for high category', () => {
    render(<RiskBadge score={85} category="high" />);
    expect(screen.getByText('85 · Danger')).toBeInTheDocument();
  });

  it('shows scoring state when score is null', () => {
    render(<RiskBadge score={null} category={null} />);
    expect(screen.getByText('— scoring')).toBeInTheDocument();
  });

  it('shows scoring state when category is null regardless of score', () => {
    render(<RiskBadge score={42} category={null} />);
    expect(screen.getByText('— scoring')).toBeInTheDocument();
  });

  it('applies low risk color style', () => {
    const { container } = render(<RiskBadge score={10} category="low" />);
    const badge = container.querySelector('span');
    expect(badge?.getAttribute('style')).toContain('rgb(0, 229, 153)');
  });

  it('applies high risk color style', () => {
    const { container } = render(<RiskBadge score={90} category="high" />);
    const badge = container.querySelector('span');
    expect(badge?.getAttribute('style')).toContain('rgb(255, 61, 90)');
  });
});
