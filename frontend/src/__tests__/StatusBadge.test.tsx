import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusBadge } from '@/components/ui/StatusBadge';

describe('StatusBadge', () => {
  it('renders label text', () => {
    render(<StatusBadge status="ok" label="Server" />);
    expect(screen.getByText('Server')).toBeInTheDocument();
  });

  it('applies green classes for "ok" status', () => {
    const { container } = render(<StatusBadge status="ok" label="Server" />);
    expect(container.firstChild).toHaveClass('text-green-400');
  });

  it('applies green classes for "connected" status', () => {
    const { container } = render(<StatusBadge status="connected" label="DB" />);
    expect(container.firstChild).toHaveClass('text-green-400');
  });

  it('applies green classes for "running" status', () => {
    const { container } = render(<StatusBadge status="running" label="Indexer" />);
    expect(container.firstChild).toHaveClass('text-green-400');
  });

  it('applies yellow classes for "idle" status', () => {
    const { container } = render(<StatusBadge status="idle" label="Indexer" />);
    expect(container.firstChild).toHaveClass('text-yellow-400');
  });

  it('applies yellow classes for "degraded" status', () => {
    const { container } = render(<StatusBadge status="degraded" label="System" />);
    expect(container.firstChild).toHaveClass('text-yellow-400');
  });

  it('applies red classes for "error" status', () => {
    const { container } = render(<StatusBadge status="error" label="DB" />);
    expect(container.firstChild).toHaveClass('text-red-400');
  });

  it('applies red classes for "failed" status', () => {
    const { container } = render(<StatusBadge status="failed" label="Redis" />);
    expect(container.firstChild).toHaveClass('text-red-400');
  });

  it('applies gray classes for unknown status', () => {
    const { container } = render(<StatusBadge status="unknown_xyz" label="Service" />);
    expect(container.firstChild).toHaveClass('text-gray-400');
  });

  it('is case-insensitive for status matching', () => {
    const { container } = render(<StatusBadge status="OK" label="Server" />);
    expect(container.firstChild).toHaveClass('text-green-400');
  });
});
