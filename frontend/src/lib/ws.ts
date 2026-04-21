import type { WsConnectionStatus, WsMessage } from './types';

function resolveWsBaseUrl(): string {
  const apiBaseUrl = process.env.NEXT_PUBLIC_API_URL ?? '/api/backend';
  const internalApiUrl = process.env.INTERNAL_API_URL;

  if (typeof window === 'undefined' && internalApiUrl) {
    if (internalApiUrl.startsWith('http://')) {
      return internalApiUrl.replace(/^http:\/\//, 'ws://');
    }
    if (internalApiUrl.startsWith('https://')) {
      return internalApiUrl.replace(/^https:\/\//, 'wss://');
    }
  }

  if (apiBaseUrl.startsWith('http://')) return apiBaseUrl.replace(/^http:\/\//, 'ws://');
  if (apiBaseUrl.startsWith('https://')) return apiBaseUrl.replace(/^https:\/\//, 'wss://');

  if (typeof window !== 'undefined') {
    const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws';
    return `${scheme}://${window.location.host}${apiBaseUrl}`;
  }

  return 'ws://backend:8080';
}

const WS_BASE_URL = resolveWsBaseUrl();

export interface WsConnection {
  close: () => void;
}

export type WsStatusCallback = (status: WsConnectionStatus) => void;
export type WsMessageCallback = (msg: WsMessage) => void;

interface WsOptions {
  onMessage: WsMessageCallback;
  onStatusChange?: WsStatusCallback;
  /** Initial reconnect delay in ms (default 1000). Doubles each retry, capped at maxDelay. */
  initialDelay?: number;
  /** Max reconnect delay in ms (default 30 000). */
  maxDelay?: number;
}

/**
 * Create a managed WebSocket connection to the MIA backend.
 *
 * Features:
 * - Auto-reconnect with exponential backoff (1s → 30s max)
 * - Responds to server Ping with Pong
 * - Parses incoming JSON as WsMessage (invalid frames are silently dropped)
 * - Returns a `close()` handle to stop reconnecting and close the socket
 */
export function createWsConnection(options: WsOptions): WsConnection {
  const {
    onMessage,
    onStatusChange,
    initialDelay = 1_000,
    maxDelay = 30_000,
  } = options;

  let ws: WebSocket | null = null;
  let delay = initialDelay;
  let stopped = false;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  function setStatus(status: WsConnectionStatus) {
    onStatusChange?.(status);
  }

  function connect() {
    if (stopped) return;

    setStatus('connecting');

    ws = new WebSocket(`${WS_BASE_URL}/ws`);

    ws.onopen = () => {
      delay = initialDelay; // reset backoff on success
      setStatus('connected');
    };

    ws.onmessage = (event: MessageEvent<string>) => {
      try {
        const msg = JSON.parse(event.data) as WsMessage;

        // Respond to server ping with pong
        if (msg.type === 'ping') {
          ws?.send(JSON.stringify({ type: 'pong' }));
          return;
        }

        onMessage(msg);
      } catch {
        // Silently ignore unparseable frames
      }
    };

    ws.onclose = () => {
      if (stopped) return;
      scheduleReconnect();
    };

    ws.onerror = () => {
      ws?.close();
    };
  }

  function scheduleReconnect() {
    setStatus('reconnecting');
    reconnectTimer = setTimeout(() => {
      delay = Math.min(delay * 2, maxDelay);
      connect();
    }, delay);
  }

  connect();

  return {
    close() {
      stopped = true;
      if (reconnectTimer !== null) clearTimeout(reconnectTimer);
      ws?.close();
      setStatus('disconnected');
    },
  };
}
