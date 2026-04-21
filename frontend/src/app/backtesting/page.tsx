'use client';

import { Fragment } from 'react';
import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import { FaArrowDown, FaArrowUp } from 'react-icons/fa6';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { api } from '@/lib/api';
import type { AlphaBacktestResponse, AlphaBacktestRowResponse } from '@/lib/types';

type WindowMode = '1h' | '6h' | 'blend';
type OutcomeBand = 'outperform' | 'neutral' | 'underperform';

type HeatCell = {
  score: number;
  count: number;
};

type HeatmapData = {
  hv: HeatCell[];
  mv: HeatCell[];
  lv: HeatCell[];
};

type VerifiedCard = {
  tokenAddress: string;
  symbol: string;
  predictedPct: number;
  actualPct: number;
  accuracyPct: number;
};

const HOURS = ['00:00', '02:00', '04:00', '06:00', '08:00', '10:00', '12:00', '14:00', '16:00', '18:00', '20:00'];

function avg(values: number[]) {
  if (values.length === 0) return 0;
  return values.reduce((a, b) => a + b, 0) / values.length;
}

function percentile(sortedValues: number[], p: number) {
  if (sortedValues.length === 0) return 0;
  const idx = Math.min(sortedValues.length - 1, Math.max(0, Math.floor((sortedValues.length - 1) * p)));
  return sortedValues[idx] ?? 0;
}

function clamp(v: number, min: number, max: number) {
  return Math.min(max, Math.max(min, v));
}

function opacityFromScore(score: number) {
  const normalized = clamp(score / 100, 0.08, 1);
  return normalized;
}

function toDisplayPct(v: number) {
  return `${v >= 0 ? '+' : ''}${v.toFixed(1)}%`;
}

function hasMatureLabel(row: AlphaBacktestRowResponse) {
  return (
    row.future_volume_1h > 0 ||
    row.future_buy_count_1h > 0 ||
    row.future_sell_count_1h > 0 ||
    row.future_volume_6h > 0 ||
    row.future_buy_count_6h > 0 ||
    row.future_sell_count_6h > 0
  );
}

function scoreForMode(row: AlphaBacktestRowResponse, mode: WindowMode) {
  if (mode === '1h') return row.score_1h;
  if (mode === '6h') return row.score_6h;
  return (row.score_1h + row.score_6h) / 2;
}

function ratioForMode(row: AlphaBacktestRowResponse, mode: WindowMode) {
  const baseline = Math.max(row.baseline_volume_1h, 0.05);
  if (mode === '1h') return row.future_volume_1h / baseline;
  if (mode === '6h') return row.future_volume_6h / baseline;
  return ((row.future_volume_1h + row.future_volume_6h) / 2) / baseline;
}

function outcomeForMode(row: AlphaBacktestRowResponse, mode: WindowMode): OutcomeBand {
  if (mode === '1h') return row.outcome_1h;
  if (mode === '6h') return row.outcome_6h;

  const weights: Record<OutcomeBand, number> = {
    outperform: 1,
    neutral: 0,
    underperform: -1,
  };

  const composite = weights[row.outcome_1h] + weights[row.outcome_6h];
  if (composite > 0) return 'outperform';
  if (composite < 0) return 'underperform';
  return 'neutral';
}

function outcomeShare(rows: AlphaBacktestRowResponse[], mode: WindowMode, outcome: OutcomeBand) {
  if (rows.length === 0) return 0;
  return (rows.filter((row) => outcomeForMode(row, mode) === outcome).length / rows.length) * 100;
}

function qualityLabel(coverage: number) {
  if (coverage >= 75) return 'High';
  if (coverage >= 45) return 'Medium';
  return 'Low';
}

function buildHeatmap(rows: AlphaBacktestRowResponse[], mode: WindowMode): HeatmapData {
  const baselineValues = rows.map((r) => r.baseline_volume_1h).sort((a, b) => a - b);
  const p33 = percentile(baselineValues, 0.33);
  const p66 = percentile(baselineValues, 0.66);

  const empty = () => Array.from({ length: HOURS.length }, () => ({ score: 0, count: 0 }));
  const hv = empty();
  const mv = empty();
  const lv = empty();

  for (const row of rows) {
    const h = new Date(row.window_end).getHours();
    const bucket = Math.floor(h / 2);
    if (bucket < 0 || bucket >= HOURS.length) continue;

    const score = scoreForMode(row, mode);

    let zone = lv;
    if (row.baseline_volume_1h >= p66) zone = hv;
    else if (row.baseline_volume_1h >= p33) zone = mv;

    zone[bucket].score += score;
    zone[bucket].count += 1;
  }

  const normalize = (cells: HeatCell[]) =>
    cells.map((c) => ({
      score: c.count > 0 ? c.score / c.count : 0,
      count: c.count,
    }));

  return { hv: normalize(hv), mv: normalize(mv), lv: normalize(lv) };
}

function buildReplayCsv(rows: AlphaBacktestRowResponse[]) {
  const headers = [
    'window_end',
    'rank',
    'token_address',
    'alpha_score',
    'baseline_volume_1h',
    'future_volume_1h',
    'future_buy_count_1h',
    'future_sell_count_1h',
    'score_1h',
    'outcome_1h',
    'future_volume_6h',
    'future_buy_count_6h',
    'future_sell_count_6h',
    'score_6h',
    'outcome_6h',
  ];

  const escape = (value: string | number) => `"${String(value).replaceAll('"', '""')}"`;
  const body = rows.map((row) =>
    [
      row.window_end,
      row.rank,
      row.token_address,
      row.alpha_score,
      row.baseline_volume_1h,
      row.future_volume_1h,
      row.future_buy_count_1h,
      row.future_sell_count_1h,
      row.score_1h,
      row.outcome_1h,
      row.future_volume_6h,
      row.future_buy_count_6h,
      row.future_sell_count_6h,
      row.score_6h,
      row.outcome_6h,
    ]
      .map(escape)
      .join(',')
  );

  return [headers.join(','), ...body].join('\n');
}

export default function BacktestingPage() {
  const [data, setData] = useState<AlphaBacktestResponse | null>(null);
  const [windowMode, setWindowMode] = useState<WindowMode>('1h');
  const [cards, setCards] = useState<VerifiedCard[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const res = await api.alpha.backtest(48, 180);
        if (!active) return;
        setData(res);
        setError(null);

        const ranked = [...res.rows]
          .filter(hasMatureLabel)
          .map((row) => {
            const predicted = clamp(row.alpha_score * 5 - 50, -100, 500);
            const actual = clamp(((row.future_volume_6h / Math.max(row.baseline_volume_1h, 0.05)) - 1) * 100, -100, 900);
            const fit = clamp(100 - (Math.abs(predicted - actual) / Math.max(40, Math.abs(predicted))) * 100, 0, 100);
            return { row, predicted, actual, fit };
          })
          .sort((a, b) => b.fit - a.fit)
          .slice(0, 8);

        const symbols = await Promise.all(
          ranked.map(async ({ row }) => {
            try {
              const token = await api.tokens.get(row.token_address);
              return [row.token_address, token.symbol ?? token.name ?? row.token_address.slice(0, 8)] as const;
            } catch {
              return [row.token_address, row.token_address.slice(0, 8)] as const;
            }
          })
        );

        const symbolMap = Object.fromEntries(symbols);
        if (!active) return;

        setCards(
          ranked.map(({ row, predicted, actual, fit }) => ({
            tokenAddress: row.token_address,
            symbol: symbolMap[row.token_address] ?? row.token_address.slice(0, 8),
            predictedPct: predicted,
            actualPct: actual,
            accuracyPct: fit,
          }))
        );
      } catch (e) {
        if (!active) return;
        setError(e instanceof Error ? e.message : 'Failed to load backtesting');
      } finally {
        if (active) setLoading(false);
      }
    }
    load();
    return () => {
      active = false;
    };
  }, []);

  const rows = useMemo(() => data?.rows ?? [], [data]);
  const matureRows = useMemo(() => rows.filter(hasMatureLabel), [rows]);
  const heatmap = useMemo(() => buildHeatmap(matureRows, windowMode), [matureRows, windowMode]);

  const matureOutperformShare = useMemo(() => outcomeShare(matureRows, windowMode, 'outperform'), [matureRows, windowMode]);
  const matureUnderperformShare = useMemo(() => outcomeShare(matureRows, windowMode, 'underperform'), [matureRows, windowMode]);
  const matureNeutralShare = useMemo(() => outcomeShare(matureRows, windowMode, 'neutral'), [matureRows, windowMode]);
  const topQuartileRows = useMemo(() => {
    if (matureRows.length === 0) return [];
    const alphaValues = matureRows.map((row) => row.alpha_score).sort((a, b) => a - b);
    const threshold = percentile(alphaValues, 0.75);
    return matureRows.filter((row) => row.alpha_score >= threshold);
  }, [matureRows]);
  const topQuartilePrecision = useMemo(() => outcomeShare(topQuartileRows, windowMode, 'outperform'), [topQuartileRows, windowMode]);

  const highScoreRows = useMemo(() => matureRows.filter((row) => scoreForMode(row, windowMode) >= 65), [matureRows, windowMode]);
  const highScoreMultiplier = useMemo(() => avg(highScoreRows.map((row) => ratioForMode(row, windowMode))), [highScoreRows, windowMode]);

  const lowScoreRows = useMemo(() => matureRows.filter((row) => scoreForMode(row, windowMode) < 45), [matureRows, windowMode]);
  const lowScoreMultiplier = useMemo(() => avg(lowScoreRows.map((row) => ratioForMode(row, windowMode))), [lowScoreRows, windowMode]);

  const matureCoverage = rows.length > 0 ? (matureRows.length / rows.length) * 100 : 0;
  const matureCoverageLabel = qualityLabel(matureCoverage);

  const topReplayWinner = useMemo(() => {
    return [...matureRows]
      .sort((a, b) => ratioForMode(b, windowMode) - ratioForMode(a, windowMode))[0] ?? null;
  }, [matureRows, windowMode]);

  const handleDownloadDataSheet = () => {
    if (rows.length === 0) return;
    const csv = buildReplayCsv(rows);
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `mia-replay-${windowMode}-${new Date().toISOString().slice(0, 10)}.csv`;
    anchor.click();
    URL.revokeObjectURL(url);
  };

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
        <section className="mb-8 flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <span className="live-dot" />
            <span
              className="mono text-xs font-bold uppercase tracking-[0.2em]"
              data-testid="proof-page-kicker"
              style={{ color: 'var(--on-surface-variant)' }}
            >
              Proof
            </span>
          </div>
          <h1 className="font-headline text-4xl font-extrabold tracking-tight" data-testid="proof-page-heading">
            Validate signal quality with replay and proof.
          </h1>
          <p className="max-w-2xl" style={{ color: 'var(--on-surface-variant)' }}>
            Use this surface to inspect replay quality, mature samples, and proof-oriented evidence that the ranking and investigation layers deserve trust.
          </p>
        </section>

        {loading && (
          <section className="glass-panel p-5 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
            Calculating backtest metrics...
          </section>
        )}

        {error && (
          <section className="glass-panel p-5 text-sm" style={{ color: 'var(--danger)' }}>
            {error}
          </section>
        )}

        {data && (
          <div className="grid grid-cols-1 gap-6 lg:grid-cols-12">
            <section className="grid grid-cols-1 gap-4 lg:col-span-12 md:grid-cols-4">
              <MetricCard label="Rows Evaluated" value={String(data.evaluated)} note="All replay rows returned by backend" />
              <MetricCard label="Mature Labels" value={String(matureRows.length)} note={`${matureCoverage.toFixed(1)}% with forward activity`} />
              <MetricCard
                label="Top-Quartile Precision"
                value={topQuartileRows.length > 0 ? `${topQuartilePrecision.toFixed(1)}%` : 'N/A'}
                note="Outperform share in the strongest mature alpha quartile"
              />
              <MetricCard
                label="Best Replay Winner"
                value={topReplayWinner ? `#${topReplayWinner.rank}` : 'N/A'}
                note={topReplayWinner ? topReplayWinner.token_address.slice(0, 10) : 'No mature sample yet'}
              />
            </section>

            <section className="rounded-xl border-l-2 p-6 shadow-xl lg:col-span-8" style={{ background: 'var(--surface-container-low)', borderLeftColor: 'rgba(183,196,255,0.3)' }}>
              <div className="mb-8 flex items-end justify-between gap-3">
                <div>
                  <h3 className="font-headline text-lg font-bold">Execution Heatmap</h3>
                  <p className="text-xs uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>
                    Mature Outcome Score by Observation Bucket
                  </p>
                </div>
                <div className="flex gap-2 rounded-lg p-1" style={{ background: 'var(--surface-container-lowest)' }}>
                  {(['1h', '6h', 'blend'] as WindowMode[]).map((w) => (
                    <button
                      key={w}
                      onClick={() => setWindowMode(w)}
                      className="rounded-md px-3 py-1 text-[10px] font-bold uppercase"
                      style={{
                        background: windowMode === w ? 'var(--primary-container)' : 'transparent',
                        color: windowMode === w ? 'var(--on-primary-container)' : 'var(--on-surface-variant)',
                      }}
                    >
                      {w === 'blend' ? 'Blend' : w}
                    </button>
                  ))}
                </div>
              </div>

              <div className="mb-4 grid grid-cols-12 gap-1">
                <div className="col-span-1" />
                <div className="col-span-11 mb-2 grid grid-cols-11 text-center text-[9px] uppercase" style={{ color: 'var(--on-surface-variant)' }}>
                  {HOURS.map((h) => (
                    <span key={h}>{h}</span>
                  ))}
                </div>

                {([
                  ['HV', heatmap.hv],
                  ['MV', heatmap.mv],
                  ['LV', heatmap.lv],
                ] as const).map(([label, cells]) => (
                  <Fragment key={label}>
                    <div key={`${label}-label`} className="col-span-1 flex items-center justify-end pr-2 text-[9px] uppercase" style={{ color: 'var(--on-surface-variant)' }}>
                      {label}
                    </div>
                    <div key={`${label}-cells`} className="col-span-11 grid grid-cols-11 gap-1">
                      {cells.map((cell, idx) => (
                        <div
                          key={`${label}-${idx}`}
                          className="h-8 rounded-[2px] border"
                          style={{
                            background:
                              cell.score >= 45
                                ? `rgba(0,255,163,${opacityFromScore(cell.score)})`
                                : `rgba(255,107,107,${opacityFromScore(100 - cell.score) * 0.8})`,
                            borderColor: 'rgba(67,70,85,0.2)',
                          }}
                          title={`score=${cell.score.toFixed(1)} count=${cell.count}`}
                        />
                      ))}
                    </div>
                  </Fragment>
                ))}
              </div>

              <div className="mt-6 flex items-center justify-between border-t pt-6" style={{ borderColor: 'rgba(67,70,85,0.2)' }}>
                <div className="flex items-center gap-6">
                  <div className="flex flex-col">
                    <span className="text-xs uppercase tracking-tighter" style={{ color: 'var(--on-surface-variant)' }}>
                      Outperform Share
                    </span>
                    <span className="mono text-2xl font-bold" style={{ color: 'var(--secondary-container)' }}>
                      {matureRows.length > 0 ? `${matureOutperformShare.toFixed(1)}%` : 'N/A'}
                    </span>
                  </div>
                  <div className="flex flex-col border-l pl-6" style={{ borderColor: 'rgba(67,70,85,0.3)' }}>
                    <span className="text-xs uppercase tracking-tighter" style={{ color: 'var(--on-surface-variant)' }}>
                      Underperform Share
                    </span>
                    <span className="mono text-2xl font-bold" style={{ color: 'var(--danger)' }}>
                      {matureRows.length > 0 ? `${matureUnderperformShare.toFixed(1)}%` : 'N/A'}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] uppercase" style={{ color: 'var(--on-surface-variant)' }}>
                    Outcome Mix:
                  </span>
                  <div className="flex h-2 w-24 overflow-hidden rounded-full" style={{ background: 'var(--surface-container-highest)' }}>
                    <div className="h-full" style={{ width: `${matureUnderperformShare}%`, background: 'rgba(255,107,107,0.9)' }} />
                    <div className="h-full" style={{ width: `${matureNeutralShare}%`, background: 'rgba(141,144,161,0.8)' }} />
                    <div className="h-full" style={{ width: `${matureOutperformShare}%`, background: 'rgba(0,255,163,0.95)' }} />
                  </div>
                  <span className="text-[10px] font-bold uppercase" style={{ color: 'var(--primary)' }}>
                    {matureCoverageLabel} sample quality
                  </span>
                </div>
              </div>
            </section>

            <aside className="relative flex flex-col justify-between overflow-hidden rounded-xl p-6 shadow-2xl lg:col-span-4" style={{ background: 'var(--surface-container-high)' }}>
              <div className="absolute right-0 top-0 h-32 w-32 rounded-full blur-[60px]" style={{ background: 'rgba(105,137,255,0.12)' }} />
              <div>
                <h3 className="font-headline mb-4 text-lg font-bold">Zone Multipliers</h3>
                <div className="space-y-6">
                  <ZoneRow
                    label="High-Score Bucket"
                    roi={highScoreMultiplier}
                    progress={highScoreRows.length > 0 ? clamp((highScoreMultiplier / 5) * 100, 5, 100) : 0}
                    hasData={highScoreRows.length > 0}
                    color="var(--secondary-container)"
                  />
                  <ZoneRow
                    label="Low-Score Bucket"
                    roi={lowScoreMultiplier}
                    progress={lowScoreRows.length > 0 ? clamp((lowScoreMultiplier / 8) * 100, 5, 100) : 0}
                    hasData={lowScoreRows.length > 0}
                    color="var(--danger)"
                  />
                </div>
              </div>

              <div className="mt-8 rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(67,70,85,0.15)' }}>
                <p className="mb-3 text-[10px] leading-relaxed" style={{ color: 'var(--on-surface-variant)' }}>
                  Zone spread reflects realized volume multiplier from mature rows only. Blend mode combines the 1H and 6H labeled windows.
                </p>
                <button
                  onClick={handleDownloadDataSheet}
                  className="w-full rounded-lg py-2 text-[10px] font-bold uppercase tracking-widest"
                  style={{ background: 'var(--surface-variant)', color: 'var(--on-surface)' }}
                >
                  Download Data Sheet
                </button>
              </div>
            </aside>

            <section className="lg:col-span-12">
              <div className="mb-4 flex items-center justify-between px-2">
                <h3 className="font-headline text-xl font-bold">Top Mature Replays</h3>
                <Link href="/alpha" className="text-xs font-bold uppercase tracking-tighter hover:underline" style={{ color: 'var(--primary)' }}>
                  View Alpha Feed
                </Link>
              </div>

              <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4">
                {cards.length > 0 ? (
                  cards.map((card) => (
                    <Link
                      key={card.tokenAddress}
                      href={`/mia?q=${encodeURIComponent(card.tokenAddress)}`}
                      className="group rounded-xl border p-5 transition-colors"
                      style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.15)' }}
                    >
                      <div className="mb-4 flex items-start justify-between">
                        <div>
                          <h4 className="font-headline text-sm font-bold tracking-tight">{card.symbol}</h4>
                          <span className="mono text-[10px] uppercase" style={{ color: 'var(--on-surface-variant)' }}>
                            {card.tokenAddress.slice(0, 10)}...
                          </span>
                        </div>
                        <div className="rounded-md px-2 py-1" style={{ background: 'rgba(0,255,163,0.1)' }}>
                          <span className="text-[9px] font-bold uppercase" style={{ color: 'var(--secondary-container)' }}>
                            {card.accuracyPct.toFixed(0)}% Fit
                          </span>
                        </div>
                      </div>

                      <div className="space-y-3">
                        <div className="flex justify-between text-[11px]">
                          <span style={{ color: 'var(--on-surface-variant)' }}>Projected Volume Lift</span>
                          <span className="mono" style={{ color: 'var(--secondary-container)' }}>{toDisplayPct(card.predictedPct)}</span>
                        </div>
                        <div className="flex justify-between text-[11px]">
                          <span style={{ color: 'var(--on-surface-variant)' }}>Realized Volume Lift</span>
                          <span className="mono">{toDisplayPct(card.actualPct)}</span>
                        </div>
                        <div className="h-1 overflow-hidden rounded-full" style={{ background: 'var(--surface-container-highest)' }}>
                          <div className="h-full" style={{ width: `${card.accuracyPct.toFixed(1)}%`, background: 'var(--primary-container)' }} />
                        </div>
                      </div>
                    </Link>
                  ))
                ) : (
                  <div className="rounded-xl border p-5 text-sm md:col-span-2 lg:col-span-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.15)', color: 'var(--on-surface-variant)' }}>
                    No mature replay cards available yet. Once more ranked observations age into 1H/6H labels, this panel will populate automatically.
                  </div>
                )}
              </div>
            </section>
          </div>
        )}
      </main>
    </>
  );
}

function MetricCard({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.15)' }}>
      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--on-surface-variant)' }}>
        {label}
      </p>
      <p className="mono mt-2 text-2xl font-bold">{value}</p>
      <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
        {note}
      </p>
    </div>
  );
}

function ZoneRow({ label, roi, progress, color, hasData }: { label: string; roi: number; progress: number; color: string; hasData: boolean }) {
  const up = roi >= 1;
  return (
    <div className="group">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="h-2 w-2 rounded-full" style={{ background: color }} />
          <span className="text-xs font-bold uppercase tracking-widest" style={{ color }}>
            {label}
          </span>
        </div>
        <span className="mono text-sm font-bold">
          {hasData ? (
            <>
              ROI {roi.toFixed(2)}x {up ? <FaArrowUp className="ml-1 inline" size={10} /> : <FaArrowDown className="ml-1 inline" size={10} />}
            </>
          ) : (
            'ROI N/A'
          )}
        </span>
      </div>
      <div className="h-12 rounded-lg p-1" style={{ background: 'var(--surface-container-lowest)' }}>
        <div className="flex h-full items-center rounded-md px-3 transition-colors" style={{ background: `${color}22` }}>
          <div className="h-1.5 rounded-full" style={{ width: `${progress}%`, background: color }} />
        </div>
      </div>
    </div>
  );
}
