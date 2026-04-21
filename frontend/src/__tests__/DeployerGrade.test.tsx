import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DeployerGrade } from '@/components/token/DeployerGrade';

describe('DeployerGrade', () => {
  it('renders grade A', () => {
    render(<DeployerGrade grade="A" />);
    expect(screen.getByText('A')).toBeInTheDocument();
  });

  it('renders grade F', () => {
    render(<DeployerGrade grade="F" />);
    expect(screen.getByText('F')).toBeInTheDocument();
  });

  it('renders optional label', () => {
    render(<DeployerGrade grade="B" label="Neutral" />);
    expect(screen.getByText('B')).toBeInTheDocument();
    expect(screen.getByText('Neutral')).toBeInTheDocument();
  });

  it('does not render label when not provided', () => {
    render(<DeployerGrade grade="C" />);
    expect(screen.queryByText('Caution')).not.toBeInTheDocument();
  });

  it('applies emerald class for grade A', () => {
    const { container } = render(<DeployerGrade grade="A" />);
    const gradeEl = container.querySelector('span span');
    expect(gradeEl?.className).toContain('emerald');
  });

  it('applies red class for grade F', () => {
    const { container } = render(<DeployerGrade grade="F" />);
    const gradeEl = container.querySelector('span span');
    expect(gradeEl?.className).toContain('red');
  });
});
