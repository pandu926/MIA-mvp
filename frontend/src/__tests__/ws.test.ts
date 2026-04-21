import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createWsConnection } from '@/lib/ws';
import type { WsMessage } from '@/lib/types';

// ─── Mock WebSocket ───────────────────────────────────────────────────────────

class MockWebSocket {
  static instances: MockWebSocket[] = [];

  url: string;
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: ((e: unknown) => void) | null = null;
  onmessage: ((e: { data: string }) => void) | null = null;
  readyState = 0;
  sentMessages: string[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sentMessages.push(data);
  }

  close() {
    this.onclose?.();
  }

  // Test helpers
  simulateOpen() {
    this.readyState = 1;
    this.onopen?.();
  }

  simulateMessage(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) });
  }

  simulateClose() {
    this.readyState = 3;
    this.onclose?.();
  }

  simulateError() {
    this.onerror?.({});
  }
}

beforeEach(() => {
  MockWebSocket.instances = [];
  vi.stubGlobal('WebSocket', MockWebSocket);
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('createWsConnection', () => {
  it('creates a WebSocket connection on call', () => {
    const conn = createWsConnection({ onMessage: vi.fn() });
    expect(MockWebSocket.instances.length).toBe(1);
    conn.close();
  });

  it('calls onStatusChange with connecting immediately', () => {
    const onStatusChange = vi.fn();
    const conn = createWsConnection({ onMessage: vi.fn(), onStatusChange });
    expect(onStatusChange).toHaveBeenCalledWith('connecting');
    conn.close();
  });

  it('calls onStatusChange with connected on open', () => {
    const onStatusChange = vi.fn();
    const conn = createWsConnection({ onMessage: vi.fn(), onStatusChange });
    MockWebSocket.instances[0].simulateOpen();
    expect(onStatusChange).toHaveBeenCalledWith('connected');
    conn.close();
  });

  it('calls onMessage with parsed WsMessage', () => {
    const onMessage = vi.fn();
    const conn = createWsConnection({ onMessage });
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    const msg: WsMessage = {
      type: 'token_update',
      token_address: '0xABC',
      name: 'Test',
      symbol: 'TST',
      deployer_address: '0xDEP',
      buy_count: 1,
      sell_count: 0,
      volume_bnb: 0.5,
      composite_score: null,
      risk_category: null,
      deployed_at: '2026-01-01T00:00:00Z',
    };
    ws.simulateMessage(msg);

    expect(onMessage).toHaveBeenCalledWith(msg);
    conn.close();
  });

  it('responds to ping with pong', () => {
    const onMessage = vi.fn();
    const conn = createWsConnection({ onMessage });
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    ws.simulateMessage({ type: 'ping' });

    expect(onMessage).not.toHaveBeenCalled(); // ping not forwarded
    expect(ws.sentMessages).toContain(JSON.stringify({ type: 'pong' }));
    conn.close();
  });

  it('silently drops unparseable frames', () => {
    const onMessage = vi.fn();
    const conn = createWsConnection({ onMessage });
    const ws = MockWebSocket.instances[0];
    ws.onmessage?.({ data: 'not-json{{' });
    expect(onMessage).not.toHaveBeenCalled();
    conn.close();
  });

  it('schedules reconnect with reconnecting status on close', () => {
    const onStatusChange = vi.fn();
    const conn = createWsConnection({ onMessage: vi.fn(), onStatusChange });
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    onStatusChange.mockClear();
    ws.simulateClose();

    expect(onStatusChange).toHaveBeenCalledWith('reconnecting');
    conn.close();
  });

  it('reconnects after initial delay', () => {
    const conn = createWsConnection({ onMessage: vi.fn(), initialDelay: 1000 });
    MockWebSocket.instances[0].simulateClose();

    expect(MockWebSocket.instances.length).toBe(1);
    vi.advanceTimersByTime(1000);
    expect(MockWebSocket.instances.length).toBe(2);
    conn.close();
  });

  it('does not reconnect after close() is called', () => {
    const conn = createWsConnection({ onMessage: vi.fn(), initialDelay: 1000 });
    const ws = MockWebSocket.instances[0];
    ws.simulateOpen();

    conn.close();
    vi.advanceTimersByTime(5000);
    expect(MockWebSocket.instances.length).toBe(1); // no new connections
  });

  it('sets disconnected status when close() is called', () => {
    const onStatusChange = vi.fn();
    const conn = createWsConnection({ onMessage: vi.fn(), onStatusChange });
    onStatusChange.mockClear();

    conn.close();

    expect(onStatusChange).toHaveBeenCalledWith('disconnected');
  });

  it('resets backoff delay on successful connection', () => {
    const conn = createWsConnection({ onMessage: vi.fn(), initialDelay: 100, maxDelay: 10_000 });

    // Simulate a close → reconnect cycle
    MockWebSocket.instances[0].simulateClose();
    vi.advanceTimersByTime(100);

    // Second connection opens → backoff should reset
    MockWebSocket.instances[1].simulateOpen();
    MockWebSocket.instances[1].simulateClose();

    // Delay should be back to 100ms (reset on open), then double to 200ms
    vi.advanceTimersByTime(100);
    expect(MockWebSocket.instances.length).toBe(3);

    conn.close();
  });
});
