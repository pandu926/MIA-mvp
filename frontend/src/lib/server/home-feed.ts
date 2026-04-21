import type { TokenListResponse } from '@/lib/types';

const SERVER_API_BASE_URL =
  process.env.INTERNAL_API_URL ??
  (process.env.NEXT_PUBLIC_API_URL?.startsWith('http') ? process.env.NEXT_PUBLIC_API_URL : null) ??
  'http://backend:8080';

async function fetchServerJson<T>(path: string): Promise<T | null> {
  try {
    const response = await fetch(`${SERVER_API_BASE_URL}${path}`, {
      cache: 'no-store',
      headers: { 'Content-Type': 'application/json' },
    });
    if (!response.ok) return null;
    return (await response.json()) as T;
  } catch {
    return null;
  }
}

export async function loadInitialHomeFeed(): Promise<TokenListResponse> {
  return (
    (await fetchServerJson<TokenListResponse>('/api/v1/tokens?limit=20&sort=newest')) ?? {
      data: [],
      total: 0,
      limit: 20,
      offset: 0,
    }
  );
}
