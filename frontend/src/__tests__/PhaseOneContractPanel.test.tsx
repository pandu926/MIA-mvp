import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PhaseOneContractPanel } from '@/components/mia/PhaseOneContractPanel';

describe('PhaseOneContractPanel', () => {
  it('renders the two-lane product contract for /mia', () => {
    render(<PhaseOneContractPanel />);

    expect(screen.getByText('Phase 1 Contract')).toBeInTheDocument();
    expect(screen.getByText('Free Workflow')).toBeInTheDocument();
    expect(screen.getByText('Deep Research')).toBeInTheDocument();
    expect(screen.getByText('Heurist Mesh upstream research')).toBeInTheDocument();
    expect(screen.getByText(/Heurist remains an upstream source/i)).toBeInTheDocument();
  });
});
