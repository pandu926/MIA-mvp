'use client';

import { Suspense } from 'react';
import { useSearchParams } from 'next/navigation';
import { AskMiaChat } from '@/components/mia/AskMiaChat';
import { ObsidianNav } from '@/components/ui/ObsidianNav';
import { shortenAddress } from '@/lib/ask-mia';

function AskMiaChatPageClient() {
  const searchParams = useSearchParams();
  const tokenAddress = searchParams.get('q')?.trim() ?? '';
  const tokenLabel = searchParams.get('label')?.trim() || shortenAddress(tokenAddress || 'token');
  const runId = searchParams.get('run')?.trim() || undefined;

  return (
    <>
      <ObsidianNav />
      <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
        {tokenAddress ? (
          <AskMiaChat tokenAddress={tokenAddress} tokenLabel={tokenLabel} runId={runId} />
        ) : (
          <section
            className="rounded-2xl border p-6"
            style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(148,160,194,0.16)' }}
          >
            <p className="text-[10px] font-bold uppercase tracking-[0.22em]" style={{ color: 'var(--outline)' }}>
              Ask MIA
            </p>
            <h1 className="mt-2 font-headline text-2xl font-bold tracking-tight">
              No token is attached yet.
            </h1>
            <p className="mt-2 max-w-2xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
              Open a token report first, then jump into the Ask MIA chat workspace from there.
            </p>
          </section>
        )}
      </main>
    </>
  );
}

export default function AskMiaChatPage() {
  return (
    <Suspense
      fallback={
        <>
          <ObsidianNav />
          <main className="obsidian-page mx-auto max-w-7xl space-y-6 px-4 md:px-8">
            <section
              className="rounded-xl border p-6 text-sm"
              style={{ background: 'var(--surface-container-low)', borderColor: 'rgba(148,160,194,0.16)', color: 'var(--on-surface-variant)' }}
            >
              Loading Ask MIA chat...
            </section>
          </main>
        </>
      }
    >
      <AskMiaChatPageClient />
    </Suspense>
  );
}
