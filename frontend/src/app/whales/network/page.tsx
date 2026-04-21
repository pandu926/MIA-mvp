'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import {
  FaBell,
  FaChevronRight,
  FaMagnifyingGlass,
  FaStar,
  FaWaveSquare,
} from 'react-icons/fa6';
import { api } from '@/lib/api';
import type { WhaleNetworkResponse, WalletIntelResponse } from '@/lib/types';

export default function WhaleNetworkPage() {
  const [walletFromQuery, setWalletFromQuery] = useState<string | null>(null);
  const [network, setNetwork] = useState<WhaleNetworkResponse | null>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [walletIntel, setWalletIntel] = useState<WalletIntelResponse | null>(null);
  const [walletLoading, setWalletLoading] = useState(false);
  const [criticalOnly, setCriticalOnly] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const res = await api.whales.network({
          hours: 24,
          min_amount: 0.5,
          ...(criticalOnly ? { level: 'critical' } : {}),
        });
        if (!active) return;
        setNetwork(res);
        setError(null);
      } catch (e) {
        if (active) setError(e instanceof Error ? e.message : 'Failed to load whale network');
      }
    };

    load();
    const id = setInterval(load, 15_000);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, [criticalOnly]);

  const selectedNode = useMemo(() => {
    if (!network || network.nodes.length === 0) return null;
    return network.nodes.find((n) => n.id === selectedNodeId) ?? network.nodes[0];
  }, [network, selectedNodeId]);
  const selectedWalletAddress =
    selectedNode?.node_type === 'wallet' ? selectedNode.wallet_address : null;

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    setWalletFromQuery(params.get('wallet')?.toLowerCase() ?? null);
  }, []);

  useEffect(() => {
    if (!network || !walletFromQuery) return;
    const match = network.nodes.find(
      (node) =>
        node.node_type === 'wallet' &&
        node.wallet_address?.toLowerCase() === walletFromQuery
    );
    if (match) setSelectedNodeId(match.id);
  }, [network, walletFromQuery]);

  useEffect(() => {
    if (!selectedWalletAddress) {
      setWalletIntel(null);
      return;
    }

    let active = true;
    setWalletLoading(true);
    api.wallets
      .intel(selectedWalletAddress, 24)
      .then((res) => {
        if (active) setWalletIntel(res);
      })
      .catch(() => {
        if (active) setWalletIntel(null);
      })
      .finally(() => {
        if (active) setWalletLoading(false);
      });

    return () => {
      active = false;
    };
  }, [selectedWalletAddress]);

  const walletNodes = useMemo(
    () => (network?.nodes ?? []).filter((n) => n.node_type === 'wallet').slice(0, 8),
    [network]
  );

  const tokenNodes = useMemo(
    () => (network?.nodes ?? []).filter((n) => n.node_type === 'token').slice(0, 8),
    [network]
  );

  const graphNodes = useMemo(() => {
    const positions = [
      { left: 45, top: 50 },
      { left: 70, top: 40 },
      { left: 82, top: 58 },
      { left: 24, top: 34 },
      { left: 28, top: 70 },
      { left: 55, top: 20 },
      { left: 12, top: 80 },
      { left: 75, top: 20 },
    ];

    const wallets = walletNodes.map((n, i) => ({ ...n, ...positions[i % positions.length] }));
    const tokens = tokenNodes.map((n, i) => ({ ...n, ...positions[(i + 3) % positions.length] }));
    return { wallets, tokens };
  }, [walletNodes, tokenNodes]);

  return (
    <div className="min-h-screen" style={{ background: 'var(--background)', color: 'var(--on-background)' }}>
      <header className="top-nav">
        <div className="mx-auto flex h-full w-full max-w-7xl items-center justify-between px-6">
          <div className="flex items-center gap-4">
            <span className="rounded-lg p-2" style={{ color: 'var(--primary)' }}>
              <FaMagnifyingGlass size={14} />
            </span>
            <Link href="/" className="font-headline text-xl font-bold tracking-tight" style={{ color: 'var(--primary)' }}>
              MIA
            </Link>
          </div>
          <div className="hidden items-center gap-6 md:flex">
            <Link href="/tokens" className="text-sm" style={{ color: 'var(--surface-container-high)' }}>Feed</Link>
            <Link href="/alpha" className="text-sm" style={{ color: 'var(--surface-container-high)' }}>Alpha</Link>
            <Link href="/whales" className="text-sm" style={{ color: 'var(--primary)' }}>Whales</Link>
            <Link href="/user" className="text-sm" style={{ color: 'var(--surface-container-high)' }}>Profile</Link>
          </div>
          <span className="rounded-lg p-2" style={{ color: 'var(--primary)' }}>
            <FaBell size={14} />
          </span>
        </div>
      </header>

      <main className="relative flex min-h-screen overflow-hidden pt-16">
        <div
          className="relative h-[calc(100vh-160px)] flex-1 md:h-[calc(100vh-64px)]"
          style={{
            backgroundImage: 'radial-gradient(circle at 2px 2px, #292a2c 1px, transparent 0)',
            backgroundSize: '40px 40px',
          }}
        >
          <div className="absolute inset-0 origin-center transition-transform" style={{ transform: `scale(${zoom})` }}>
            <svg className="pointer-events-none absolute inset-0 h-full w-full opacity-40">
              {network?.edges.slice(0, 60).map((edge) => {
                const from = graphNodes.wallets.find((w) => w.id === edge.source);
                const to = graphNodes.tokens.find((t) => t.id === edge.target);
                if (!from || !to) return null;
                return (
                  <line
                    key={`${edge.source}-${edge.target}`}
                    x1={`${from.left}%`}
                    y1={`${from.top}%`}
                    x2={`${to.left}%`}
                    y2={`${to.top}%`}
                    stroke={edge.tx_count > 2 ? '#ffb4ab' : '#b7c4ff'}
                    strokeDasharray={edge.tx_count > 2 ? '2' : '4'}
                    strokeWidth={1}
                  />
                );
              })}
            </svg>

            {graphNodes.wallets.map((n) => {
              const active = selectedNode?.id === n.id;
              return (
                <button
                  key={n.id}
                  onClick={() => setSelectedNodeId(n.id)}
                  className="absolute -translate-x-1/2 -translate-y-1/2"
                  style={{ left: `${n.left}%`, top: `${n.top}%` }}
                >
                  <div
                    className="flex h-14 w-14 items-center justify-center rounded-lg border backdrop-blur-md"
                    style={{
                      background: 'rgba(105,137,255,0.2)',
                      borderColor: active ? 'var(--primary)' : 'rgba(183,196,255,0.4)',
                      boxShadow: active ? '0 0 30px rgba(105,137,255,0.35)' : '0 0 22px rgba(105,137,255,0.25)',
                    }}
                  >
                    <FaWaveSquare size={20} style={{ color: 'var(--primary)' }} />
                  </div>
                  <div className="mono absolute left-1/2 top-[108%] -translate-x-1/2 rounded bg-[var(--surface-container-high)] px-2 py-1 text-[10px] whitespace-nowrap">
                    {n.wallet_address?.slice(0, 8)}...
                  </div>
                </button>
              );
            })}

            {graphNodes.tokens.map((n) => {
              const active = selectedNode?.id === n.id;
              return (
                <button
                  key={n.id}
                  onClick={() => setSelectedNodeId(n.id)}
                  className="absolute -translate-x-1/2 -translate-y-1/2"
                  style={{ left: `${n.left}%`, top: `${n.top}%` }}
                >
                  <div
                    className="flex h-10 w-10 items-center justify-center rounded-lg border backdrop-blur-md"
                    style={{
                      background: 'rgba(0,255,163,0.1)',
                      borderColor: active ? 'var(--secondary-container)' : 'rgba(0,255,163,0.3)',
                    }}
                  >
                    <FaStar size={12} style={{ color: 'var(--secondary-container)' }} />
                  </div>
                  <div className="mono absolute left-1/2 top-[108%] -translate-x-1/2 rounded bg-[var(--surface-container-high)] px-2 py-1 text-[10px] whitespace-nowrap">
                    {n.token_address?.slice(0, 8)}...
                  </div>
                </button>
              );
            })}
          </div>

          <div className="absolute bottom-8 left-8 rounded-xl border p-4" style={{ background: 'rgba(27,28,30,0.8)', borderColor: 'rgba(67,70,85,0.25)' }}>
            <h3 className="mb-3 text-[10px] font-bold uppercase tracking-widest" style={{ color: 'var(--on-surface-variant)' }}>
              Entity Legend
            </h3>
            <Legend color="var(--primary)" text="WHALE ENTITY" />
            <Legend color="var(--secondary-container)" text="TOKEN NODE" />
          </div>

          <div className="absolute right-8 top-8 flex flex-col gap-2">
            <button
              onClick={() => setZoom((z) => Math.min(1.8, Number((z + 0.1).toFixed(2))))}
              className="h-10 w-10 rounded-lg border"
              style={{ background: 'rgba(52,53,55,0.6)', borderColor: 'rgba(67,70,85,0.25)' }}
            >
              +
            </button>
            <button
              onClick={() => setZoom((z) => Math.max(0.7, Number((z - 0.1).toFixed(2))))}
              className="h-10 w-10 rounded-lg border"
              style={{ background: 'rgba(52,53,55,0.6)', borderColor: 'rgba(67,70,85,0.25)' }}
            >
              -
            </button>
            <button
              onClick={() => setCriticalOnly((v) => !v)}
              className="h-10 w-10 rounded-lg border text-xs"
              style={{
                background: criticalOnly ? 'rgba(255,107,107,0.2)' : 'rgba(52,53,55,0.6)',
                borderColor: criticalOnly ? 'rgba(255,107,107,0.4)' : 'rgba(67,70,85,0.25)',
                color: criticalOnly ? 'var(--danger)' : 'var(--on-background)',
              }}
              title="Toggle critical only"
            >
              F
            </button>
          </div>
        </div>

        <aside className="hidden w-96 flex-col overflow-y-auto border-l lg:flex" style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(67,70,85,0.2)' }}>
          <div className="p-6">
            <div className="mb-6 flex items-start justify-between">
              <div>
                <h2 className="font-headline text-2xl font-extrabold tracking-tight" style={{ color: 'var(--primary-fixed-dim, var(--primary))' }}>
                  Wallet Intel
                </h2>
                <p className="mono text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                  {selectedNode?.wallet_address ?? selectedNode?.token_address ?? 'No node selected'}
                </p>
              </div>
              <span className="rounded border px-2 py-1 text-[10px] font-bold uppercase" style={{ color: 'var(--primary)', background: 'rgba(105,137,255,0.1)', borderColor: 'rgba(105,137,255,0.2)' }}>
                {selectedNode?.node_type === 'wallet' ? 'Whale Wallet' : 'Token Node'}
              </span>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <Panel
                label="Network Flow"
                value={`$${((network?.metrics.total_volume_bnb ?? 0) * 595).toFixed(0)}`}
              />
              <Panel
                label="Critical Edges"
                value={`${network?.metrics.critical_edges ?? 0}`}
                tone="var(--secondary-container)"
              />
            </div>

            {walletLoading && (
              <p className="mt-4 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
                Loading wallet intel...
              </p>
            )}

            {selectedNode?.node_type === 'wallet' && walletIntel && (
              <>
                <div className="mt-6 rounded-xl border p-5" style={{ background: 'rgba(52,53,55,0.2)', borderColor: 'rgba(67,70,85,0.2)' }}>
                  <h4 className="mb-4 text-xs font-bold uppercase tracking-widest">Recent Token Exposure</h4>
                  {walletIntel.top_tokens.slice(0, 5).map((t) => (
                    <div key={t.token_address} className="mb-3 flex items-center justify-between">
                      <div>
                        <div className="text-sm font-bold">{t.token_address.slice(0, 8)}...</div>
                        <div className="mono text-[10px]" style={{ color: 'var(--on-surface-variant)' }}>
                          {t.tx_count} tx
                        </div>
                      </div>
                      <div className="mono text-xs font-bold" style={{ color: 'var(--secondary-container)' }}>
                        {t.volume_bnb.toFixed(2)} BNB
                      </div>
                    </div>
                  ))}
                </div>

                <div className="mt-6 rounded-xl border p-4" style={{ background: 'rgba(52,53,55,0.2)', borderColor: 'rgba(67,70,85,0.2)' }}>
                  <p className="mb-2 text-xs font-bold uppercase tracking-widest">Selected Node Actions</p>
                  <div className="flex gap-2">
                    {walletIntel.top_tokens[0] && (
                      <Link href={`/mia?q=${encodeURIComponent(walletIntel.top_tokens[0].token_address)}`} className="rounded px-3 py-2 text-xs font-bold" style={{ background: 'rgba(105,137,255,0.2)', color: 'var(--primary)' }}>
                        Open Token <FaChevronRight className="ml-1 inline" size={10} />
                      </Link>
                    )}
                    <Link href={`/deployer/${walletIntel.wallet_address}`} className="rounded px-3 py-2 text-xs font-bold" style={{ background: 'rgba(0,255,163,0.14)', color: 'var(--secondary-container)' }}>
                      Wallet Profile <FaChevronRight className="ml-1 inline" size={10} />
                    </Link>
                  </div>
                </div>
              </>
            )}

            {selectedNode?.node_type === 'token' && selectedNode.token_address && (
              <div className="mt-6 rounded-xl border p-4" style={{ background: 'rgba(52,53,55,0.2)', borderColor: 'rgba(67,70,85,0.2)' }}>
                <p className="mb-2 text-xs font-bold uppercase tracking-widest">Token Actions</p>
                <Link href={`/mia?q=${encodeURIComponent(selectedNode.token_address)}`} className="rounded px-3 py-2 text-xs font-bold" style={{ background: 'rgba(105,137,255,0.2)', color: 'var(--primary)' }}>
                  Open Token <FaChevronRight className="ml-1 inline" size={10} />
                </Link>
              </div>
            )}

            {error && (
              <p className="mt-3 text-xs" style={{ color: 'var(--danger)' }}>
                {error}
              </p>
            )}
          </div>
        </aside>
      </main>

    </div>
  );
}

function Panel({ label, value, tone }: { label: string; value: string; tone?: string }) {
  return (
    <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-highest)', borderColor: 'rgba(67,70,85,0.12)' }}>
      <span className="text-[10px] uppercase font-bold" style={{ color: 'var(--on-surface-variant)' }}>
        {label}
      </span>
      <div className="mono mt-1 text-lg font-bold" style={{ color: tone ?? 'var(--on-background)' }}>
        {value}
      </div>
    </div>
  );
}

function Legend({ color, text }: { color: string; text: string }) {
  return (
    <div className="mb-2 flex items-center gap-3">
      <div className="h-2 w-2 rounded-full" style={{ background: color }} />
      <span className="mono text-xs">{text}</span>
    </div>
  );
}
