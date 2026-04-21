import { create } from 'zustand';
import type { TokenSummary, WsConnectionStatus, WsNarrativeUpdate, WsTokenUpdate } from '@/lib/types';

export interface NarrativeEntry {
  token_address: string;
  narrative_text: string;
  risk_interpretation: string | null;
  consensus_status: 'agreed' | 'diverged' | 'single_model';
  confidence: 'high' | 'medium' | 'low';
}

interface TokenFeedState {
  tokens: Map<string, TokenSummary>;
  narratives: Map<string, NarrativeEntry>;
  connectionStatus: WsConnectionStatus;
  updateToken: (update: WsTokenUpdate) => void;
  updateNarrative: (update: WsNarrativeUpdate) => void;
  setConnectionStatus: (status: WsConnectionStatus) => void;
}

export const useTokenFeedStore = create<TokenFeedState>((set) => ({
  tokens: new Map(),
  narratives: new Map(),
  connectionStatus: 'disconnected',

  updateToken: (update: WsTokenUpdate) => {
    set((state) => {
      const token: TokenSummary = {
        contract_address: update.token_address,
        name: update.name,
        symbol: update.symbol,
        deployer_address: update.deployer_address,
        deployed_at: update.deployed_at,
        block_number: state.tokens.get(update.token_address)?.block_number ?? 0,
        buy_count: update.buy_count,
        sell_count: update.sell_count,
        total_tx: update.buy_count + update.sell_count,
        volume_bnb: update.volume_bnb,
        composite_score: update.composite_score,
        risk_category: update.risk_category,
        ai_scored: false,
        deep_researched: false,
        watching_for: 'Watching for enough activity to unlock a live AI score.',
      };
      const next = new Map(state.tokens);
      next.set(update.token_address, token);
      return { tokens: next };
    });
  },

  updateNarrative: (update: WsNarrativeUpdate) => {
    set((state) => {
      const entry: NarrativeEntry = {
        token_address: update.token_address,
        narrative_text: update.narrative_text,
        risk_interpretation: update.risk_interpretation,
        consensus_status: update.consensus_status,
        confidence: update.confidence,
      };
      const next = new Map(state.narratives);
      next.set(update.token_address, entry);
      return { narratives: next };
    });
  },

  setConnectionStatus: (status: WsConnectionStatus) => {
    set({ connectionStatus: status });
  },
}));
