'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import type { WhaleAlertResponse } from '@/lib/types';

type Node = {
  id: string;
  label: string;
  kind: 'token' | 'wallet';
  x: number;
  y: number;
  vx: number;
  vy: number;
};

type Edge = { source: string; target: string };

interface WhaleGraphProps {
  alerts: WhaleAlertResponse[];
  width?: number;
  height?: number;
}

function hash32(input: string): number {
  let h = 2166136261;
  for (let i = 0; i < input.length; i += 1) {
    h ^= input.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function seededPosition(id: string, width: number, height: number): { x: number; y: number } {
  const hx = hash32(`${id}:x`) / 0xffffffff;
  const hy = hash32(`${id}:y`) / 0xffffffff;
  const margin = 24;
  return {
    x: margin + hx * (width - margin * 2),
    y: margin + hy * (height - margin * 2),
  };
}

export function WhaleGraph({ alerts, width = 900, height = 420 }: WhaleGraphProps) {
  const [tick, setTick] = useState(0);
  const nodesRef = useRef<Node[]>([]);
  const edgesRef = useRef<Edge[]>([]);

  const graph = useMemo(() => {
    const nodeMap = new Map<string, Node>();
    const edges: Edge[] = [];

    for (const a of alerts) {
      const tokenId = `token:${a.token_address}`;
      const walletId = `wallet:${a.wallet_address}`;
      if (!nodeMap.has(tokenId)) {
        const pos = seededPosition(tokenId, width, height);
        nodeMap.set(tokenId, {
          id: tokenId,
          label: a.token_address,
          kind: 'token',
          x: pos.x,
          y: pos.y,
          vx: 0,
          vy: 0,
        });
      }
      if (!nodeMap.has(walletId)) {
        const pos = seededPosition(walletId, width, height);
        nodeMap.set(walletId, {
          id: walletId,
          label: a.wallet_address,
          kind: 'wallet',
          x: pos.x,
          y: pos.y,
          vx: 0,
          vy: 0,
        });
      }
      edges.push({ source: walletId, target: tokenId });
    }

    return { nodes: [...nodeMap.values()], edges };
  }, [alerts, width, height]);

  useEffect(() => {
    nodesRef.current = graph.nodes;
    edgesRef.current = graph.edges;
    setTick((v) => v + 1);
  }, [graph]);

  useEffect(() => {
    let raf = 0;
    const run = () => {
      const nodes = nodesRef.current;
      const edges = edgesRef.current;
      if (nodes.length === 0) return;

      // Simple force simulation: repulsion + edge spring + center gravity.
      for (let i = 0; i < nodes.length; i += 1) {
        const a = nodes[i];
        for (let j = i + 1; j < nodes.length; j += 1) {
          const b = nodes[j];
          let dx = a.x - b.x;
          let dy = a.y - b.y;
          const d2 = Math.max(dx * dx + dy * dy, 36);
          const force = 180 / d2;
          dx *= force;
          dy *= force;
          a.vx += dx;
          a.vy += dy;
          b.vx -= dx;
          b.vy -= dy;
        }
      }

      for (const e of edges) {
        const s = nodes.find((n) => n.id === e.source);
        const t = nodes.find((n) => n.id === e.target);
        if (!s || !t) continue;
        const dx = t.x - s.x;
        const dy = t.y - s.y;
        const len = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
        const spring = (len - 90) * 0.0022;
        const fx = (dx / len) * spring;
        const fy = (dy / len) * spring;
        s.vx += fx;
        s.vy += fy;
        t.vx -= fx;
        t.vy -= fy;
      }

      for (const n of nodes) {
        n.vx += (width * 0.5 - n.x) * 0.0006;
        n.vy += (height * 0.5 - n.y) * 0.0006;
        n.vx *= 0.9;
        n.vy *= 0.9;
        n.x = Math.max(14, Math.min(width - 14, n.x + n.vx * 14));
        n.y = Math.max(14, Math.min(height - 14, n.y + n.vy * 14));
      }

      setTick((v) => v + 1);
      raf = requestAnimationFrame(run);
    };
    raf = requestAnimationFrame(run);
    return () => cancelAnimationFrame(raf);
  }, [height, width]);

  const nodes = nodesRef.current;
  const edges = edgesRef.current;

  return (
    <svg key={tick} viewBox={`0 0 ${width} ${height}`} className="w-full h-full rounded-xl">
      <rect x="0" y="0" width={width} height={height} fill="#f8fbff" />
      {edges.map((e, i) => {
        const s = nodes.find((n) => n.id === e.source);
        const t = nodes.find((n) => n.id === e.target);
        if (!s || !t) return null;
        return (
          <line
            key={`${e.source}-${e.target}-${i}`}
            x1={s.x}
            y1={s.y}
            x2={t.x}
            y2={t.y}
            stroke="rgba(137,153,178,0.45)"
            strokeWidth="1"
          />
        );
      })}
      {nodes.map((n) => (
        <g key={n.id}>
          <circle
            cx={n.x}
            cy={n.y}
            r={n.kind === 'token' ? 8 : 6}
            fill={n.kind === 'token' ? '#0f6fff' : '#d18700'}
          />
          <text x={n.x + 10} y={n.y + 4} fontSize="10" fill="#4f617d">
            {n.label}
          </text>
        </g>
      ))}
    </svg>
  );
}
