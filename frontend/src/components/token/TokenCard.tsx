import { FaArrowDown, FaArrowUp, FaBrain, FaTriangleExclamation } from 'react-icons/fa6';
import type { TokenSummary } from '@/lib/types';
import type { NarrativeEntry } from '@/stores/token-feed';

interface TokenCardProps {
  token: TokenSummary;
  narrative?: NarrativeEntry;
}

function timeAgo(iso: string) {
  const diff = Date.now() - new Date(iso).getTime();
  const m = Math.floor(diff / 60_000);
  if (m < 1) return 'now';
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  return `${Math.floor(h / 24)}d`;
}

function riskTone(risk: TokenSummary['risk_category']) {
  if (risk === 'low') return { className: 'safe', label: 'Low Risk' };
  if (risk === 'medium') return { className: 'warn', label: 'Mid Risk' };
  if (risk === 'high') return { className: 'danger', label: 'High Risk' };
  return { className: 'warn', label: 'Scoring' };
}

export function TokenCard({ token, narrative }: TokenCardProps) {
  const total = token.buy_count + token.sell_count;
  const buyPct = total > 0 ? (token.buy_count / total) * 100 : 0;
  const tone = riskTone(token.risk_category);

  return (
    <article className="group relative overflow-hidden rounded-xl border p-4 transition-transform hover:translate-x-1" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(167,169,178,0.14)' }}>
      <div className="absolute -right-12 -top-12 h-36 w-36 rounded-full blur-3xl" style={{ background: 'rgba(105,137,255,0.18)' }} />

      <div className="relative flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="font-headline text-base font-extrabold tracking-tight">{token.symbol ?? token.name ?? 'Unknown'}</p>
          <p className="mono mt-1 break-all text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
            {token.contract_address}
          </p>
        </div>
        <span className={`signal-chip ${tone.className}`}>{tone.label}</span>
      </div>

      <div className="mt-3 grid grid-cols-3 gap-2 text-xs">
        <Stat label="Volume" value={`${token.volume_bnb.toFixed(3)} B`} />
        <Stat label="Buys" value={String(token.buy_count)} />
        <Stat label="Sells" value={String(token.sell_count)} />
      </div>

      <div className="mt-3">
        <div className="mb-1 flex items-center justify-between text-[11px]">
          <span className="inline-flex items-center gap-1" style={{ color: 'var(--secondary-container)' }}>
            <FaArrowUp size={9} /> {buyPct.toFixed(0)}%
          </span>
          <span className="inline-flex items-center gap-1" style={{ color: 'var(--danger)' }}>
            {(100 - buyPct).toFixed(0)}% <FaArrowDown size={9} />
          </span>
        </div>
        <div className="h-1.5 overflow-hidden rounded-full" style={{ background: 'var(--outline-variant)' }}>
          <div className="h-full rounded-full" style={{ width: `${buyPct}%`, background: 'var(--primary-container)' }} />
        </div>
      </div>

      <div className="mt-3 flex items-center justify-between text-[11px]" style={{ color: 'var(--on-surface-variant)' }}>
        <span className="num">Score {token.composite_score ?? '—'}</span>
        <span className="num">{timeAgo(token.deployed_at)}</span>
      </div>

      <div className="mt-3 rounded-lg border p-2.5" style={{ borderColor: 'rgba(105,137,255,0.3)', background: 'rgba(105,137,255,0.08)' }}>
        <p className="mb-1 text-[11px] font-semibold" style={{ color: 'var(--primary)' }}>
          <FaBrain className="mr-1 inline" size={9} /> AI Narrative
        </p>
        <p className="text-xs leading-relaxed" style={{ color: 'var(--on-surface-variant)' }}>
          {narrative?.narrative_text?.slice(0, 120) ?? 'Waiting for enough signal density before generating the AI consensus.'}
          {(narrative?.narrative_text?.length ?? 0) > 120 ? '…' : ''}
        </p>
      </div>

      {token.risk_category === 'high' && (
        <p className="mt-2 text-[11px] font-semibold" style={{ color: 'var(--danger)' }}>
          <FaTriangleExclamation className="mr-1 inline" size={9} /> Elevated risk structure detected.
        </p>
      )}
    </article>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg px-2 py-1.5" style={{ background: 'var(--surface-container-high)' }}>
      <p className="text-[10px] uppercase tracking-wider" style={{ color: 'var(--on-surface-variant)' }}>
        {label}
      </p>
      <p className="num mt-1 text-xs font-semibold">{value}</p>
    </div>
  );
}
