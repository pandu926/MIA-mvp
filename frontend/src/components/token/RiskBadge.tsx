import { Circle } from 'lucide-react';
import type { RiskCategory } from '@/lib/types';

interface RiskBadgeProps {
  score: number | null;
  category: RiskCategory | null;
  size?: 'sm' | 'md';
}

const CONFIG: Record<RiskCategory, { label: string; color: string; bg: string; border: string }> = {
  low:    { label: 'Safe',    color: '#00e599', bg: 'rgba(0,229,153,0.08)',  border: 'rgba(0,229,153,0.25)'  },
  medium: { label: 'Caution', color: '#ffb020', bg: 'rgba(255,176,32,0.08)', border: 'rgba(255,176,32,0.25)' },
  high:   { label: 'Danger',  color: '#ff3d5a', bg: 'rgba(255,61,90,0.08)',  border: 'rgba(255,61,90,0.25)'  },
};

export function RiskBadge({ score, category, size = 'md' }: RiskBadgeProps) {
  if (score === null || category === null) {
    return (
      <span
        className="inline-flex items-center gap-1 rounded font-medium num"
        style={{
          padding: size === 'sm' ? '2px 8px' : '3px 10px',
          fontSize: '0.7rem',
          background: '#0c1220',
          border: '1px solid #1a2840',
          color: '#3d5470',
        }}
      >
        — scoring
      </span>
    );
  }

  const c = CONFIG[category];
  return (
    <span
      className="inline-flex items-center gap-1.5 rounded font-semibold num"
      style={{
        padding: size === 'sm' ? '2px 8px' : '3px 10px',
        fontSize: size === 'sm' ? '0.7rem' : '0.72rem',
        background: c.bg,
        border: `1px solid ${c.border}`,
        color: c.color,
      }}
    >
      <Circle size={6} fill={c.color} color={c.color} />
      {score} · {c.label}
    </span>
  );
}
