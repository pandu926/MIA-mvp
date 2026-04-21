import { Bot, TrendingUp } from 'lucide-react';
import type { WsNarrativeUpdate } from '@/lib/types';

interface NarrativeCardProps {
  narrative: WsNarrativeUpdate;
}

const CONSENSUS = {
  agreed: {
    label: 'AI Agreed',
    color: '#00e599',
    bg: 'rgba(0,229,153,0.08)',
    border: 'rgba(0,229,153,0.2)',
  },
  diverged: {
    label: 'AI Diverged',
    color: '#ffb020',
    bg: 'rgba(255,176,32,0.08)',
    border: 'rgba(255,176,32,0.2)',
  },
  single_model: {
    label: 'Single Model',
    color: '#5a7490',
    bg: 'rgba(90,116,144,0.08)',
    border: 'rgba(90,116,144,0.2)',
  },
};

const CONFIDENCE = {
  high:   { label: 'High',   color: '#00e599' },
  medium: { label: 'Medium', color: '#ffb020' },
  low:    { label: 'Low',    color: '#5a7490' },
};

export function NarrativeCard({ narrative }: NarrativeCardProps) {
  const c = CONSENSUS[narrative.consensus_status];
  const f = CONFIDENCE[narrative.confidence];

  return (
    <div
      className="mt-3 rounded-xl p-3.5"
      style={{ background: 'rgba(167,139,250,0.05)', border: '1px solid rgba(167,139,250,0.2)' }}
    >
      {/* Header */}
      <div className="flex items-center justify-between gap-2 mb-3">
        <span className="flex items-center gap-1.5 text-xs font-semibold" style={{ color: '#a78bfa' }}>
          <Bot size={13} />
          AI Analysis
        </span>
        <div className="flex items-center gap-2">
          <span
            className="text-xs px-2 py-0.5 rounded font-medium"
            style={{ background: c.bg, border: `1px solid ${c.border}`, color: c.color }}
          >
            {c.label}
          </span>
          <span className="text-xs flex items-center gap-1" style={{ color: f.color }}>
            <TrendingUp size={10} />
            {f.label}
          </span>
        </div>
      </div>

      {/* Narrative text */}
      <p className="text-xs leading-relaxed" style={{ color: '#8aa0b8' }}>
        {narrative.narrative_text}
      </p>

      {/* Risk interpretation */}
      {narrative.risk_interpretation && (
        <div
          className="mt-2.5 pt-2.5 text-xs leading-relaxed"
          style={{ borderTop: '1px solid rgba(167,139,250,0.12)', color: '#5a7490' }}
        >
          {narrative.risk_interpretation}
        </div>
      )}
    </div>
  );
}
