import { describe, it, expect, beforeEach } from 'vitest';
import { useTokenFeedStore } from '@/stores/token-feed';
import type { WsTokenUpdate, WsNarrativeUpdate } from '@/lib/types';

// Helper to reset Zustand store between tests
function resetStore() {
  useTokenFeedStore.setState({
    tokens: new Map(),
    narratives: new Map(),
    connectionStatus: 'disconnected',
  });
}

describe('useTokenFeedStore', () => {
  beforeEach(() => {
    resetStore();
  });

  // ─── Initial state ──────────────────────────────────────────────────────────

  it('starts with empty tokens and narratives maps', () => {
    const state = useTokenFeedStore.getState();
    expect(state.tokens.size).toBe(0);
    expect(state.narratives.size).toBe(0);
  });

  it('starts with disconnected status', () => {
    const { connectionStatus } = useTokenFeedStore.getState();
    expect(connectionStatus).toBe('disconnected');
  });

  // ─── updateToken ────────────────────────────────────────────────────────────

  it('adds a new token on updateToken', () => {
    const update: WsTokenUpdate = {
      type: 'token_update',
      token_address: '0xABC',
      name: 'MyToken',
      symbol: 'MTK',
      deployer_address: '0xDEP',
      buy_count: 5,
      sell_count: 2,
      volume_bnb: 1.5,
      composite_score: 42,
      risk_category: 'medium',
      deployed_at: '2026-01-01T00:00:00Z',
    };

    useTokenFeedStore.getState().updateToken(update);

    const { tokens } = useTokenFeedStore.getState();
    expect(tokens.size).toBe(1);
    const token = tokens.get('0xABC');
    expect(token).toBeDefined();
    expect(token?.symbol).toBe('MTK');
    expect(token?.buy_count).toBe(5);
  });

  it('updates an existing token immutably on updateToken', () => {
    const first: WsTokenUpdate = {
      type: 'token_update',
      token_address: '0xABC',
      name: 'MyToken',
      symbol: 'MTK',
      deployer_address: '0xDEP',
      buy_count: 5,
      sell_count: 2,
      volume_bnb: 1.5,
      composite_score: 42,
      risk_category: 'medium',
      deployed_at: '2026-01-01T00:00:00Z',
    };
    useTokenFeedStore.getState().updateToken(first);
    const mapBefore = useTokenFeedStore.getState().tokens;

    const second: WsTokenUpdate = { ...first, buy_count: 10, volume_bnb: 3.0 };
    useTokenFeedStore.getState().updateToken(second);

    const { tokens } = useTokenFeedStore.getState();
    // Map reference must be a new object (immutable update)
    expect(tokens).not.toBe(mapBefore);
    expect(tokens.size).toBe(1);
    expect(tokens.get('0xABC')?.buy_count).toBe(10);
    expect(tokens.get('0xABC')?.volume_bnb).toBe(3.0);
  });

  it('handles multiple different tokens', () => {
    const makeUpdate = (addr: string): WsTokenUpdate => ({
      type: 'token_update',
      token_address: addr,
      name: null,
      symbol: null,
      deployer_address: '0xDEP',
      buy_count: 1,
      sell_count: 0,
      volume_bnb: 0.1,
      composite_score: null,
      risk_category: null,
      deployed_at: '2026-01-01T00:00:00Z',
    });

    useTokenFeedStore.getState().updateToken(makeUpdate('0xA'));
    useTokenFeedStore.getState().updateToken(makeUpdate('0xB'));
    useTokenFeedStore.getState().updateToken(makeUpdate('0xC'));

    expect(useTokenFeedStore.getState().tokens.size).toBe(3);
  });

  // ─── updateNarrative ────────────────────────────────────────────────────────

  it('adds a narrative on updateNarrative', () => {
    const update: WsNarrativeUpdate = {
      type: 'narrative_update',
      token_address: '0xABC',
      narrative_text: 'Looks organic',
      risk_interpretation: 'Low risk profile',
      consensus_status: 'agreed',
      confidence: 'high',
    };

    useTokenFeedStore.getState().updateNarrative(update);

    const { narratives } = useTokenFeedStore.getState();
    expect(narratives.size).toBe(1);
    const n = narratives.get('0xABC');
    expect(n?.narrative_text).toBe('Looks organic');
    expect(n?.consensus_status).toBe('agreed');
  });

  it('replaces existing narrative immutably', () => {
    const first: WsNarrativeUpdate = {
      type: 'narrative_update',
      token_address: '0xABC',
      narrative_text: 'First narrative',
      risk_interpretation: null,
      consensus_status: 'single_model',
      confidence: 'low',
    };
    useTokenFeedStore.getState().updateNarrative(first);
    const mapBefore = useTokenFeedStore.getState().narratives;

    const second: WsNarrativeUpdate = {
      ...first,
      narrative_text: 'Updated narrative',
      consensus_status: 'agreed',
      confidence: 'high',
    };
    useTokenFeedStore.getState().updateNarrative(second);

    const { narratives } = useTokenFeedStore.getState();
    expect(narratives).not.toBe(mapBefore);
    expect(narratives.get('0xABC')?.narrative_text).toBe('Updated narrative');
  });

  // ─── setConnectionStatus ───────────────────────────────────────────────────

  it('updates connectionStatus', () => {
    useTokenFeedStore.getState().setConnectionStatus('connecting');
    expect(useTokenFeedStore.getState().connectionStatus).toBe('connecting');

    useTokenFeedStore.getState().setConnectionStatus('connected');
    expect(useTokenFeedStore.getState().connectionStatus).toBe('connected');

    useTokenFeedStore.getState().setConnectionStatus('reconnecting');
    expect(useTokenFeedStore.getState().connectionStatus).toBe('reconnecting');

    useTokenFeedStore.getState().setConnectionStatus('disconnected');
    expect(useTokenFeedStore.getState().connectionStatus).toBe('disconnected');
  });

  // ─── WsTokenUpdate → TokenSummary mapping ──────────────────────────────────

  it('maps WsTokenUpdate fields to TokenSummary shape correctly', () => {
    const update: WsTokenUpdate = {
      type: 'token_update',
      token_address: '0xCONTRACT',
      name: 'Alpha',
      symbol: 'ALP',
      deployer_address: '0xDEPLOYER',
      buy_count: 7,
      sell_count: 3,
      volume_bnb: 2.5,
      composite_score: 80,
      risk_category: 'low',
      deployed_at: '2026-04-01T12:00:00Z',
    };

    useTokenFeedStore.getState().updateToken(update);
    const token = useTokenFeedStore.getState().tokens.get('0xCONTRACT');

    expect(token?.contract_address).toBe('0xCONTRACT');
    expect(token?.name).toBe('Alpha');
    expect(token?.symbol).toBe('ALP');
    expect(token?.deployer_address).toBe('0xDEPLOYER');
    expect(token?.buy_count).toBe(7);
    expect(token?.sell_count).toBe(3);
    expect(token?.volume_bnb).toBe(2.5);
    expect(token?.composite_score).toBe(80);
    expect(token?.risk_category).toBe('low');
    expect(token?.deployed_at).toBe('2026-04-01T12:00:00Z');
  });
});
