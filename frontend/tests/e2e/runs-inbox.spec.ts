import { expect, test } from '@playwright/test';

const nonTransactionEscalationScenarios = [
  {
    signal: 'wallet_concentration',
    badge: /wallet concentration/i,
    reason: /wallet concentration/i,
  },
  {
    signal: 'whale_alert',
    badge: /whale alert/i,
    reason: /critical whale alert/i,
  },
  {
    signal: 'builder_overlap',
    badge: /builder overlap/i,
    reason: /builder overlap/i,
  },
  {
    signal: 'linked_launch_overlap',
    badge: /linked launch overlap/i,
    reason: /linked launch overlap/i,
  },
] as const;

const monitoringDowngradeScenarios = [
  {
    signal: 'activity',
    badge: /activity/i,
    reason: /Auto monitoring downgrade reason:/i,
  },
  {
    signal: 'linked_launch_overlap',
    badge: /linked launch overlap/i,
    reason: /linked launch overlap cooled/i,
  },
  {
    signal: 'source_degradation',
    badge: /source degradation/i,
    reason: /source health degraded/i,
  },
] as const;

async function waitForEscalatedRun(
  request: import('@playwright/test').APIRequestContext,
  tokenAddress: string,
  signal: string,
  maxAttempts = 3
) {
  let lastPayload:
    | {
        escalated_runs?: Array<{ run_id: string; token_address: string; signal_tag: string; reason: string }>;
      }
    | undefined;

  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    const scanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
    expect(scanResponse.ok()).toBeTruthy();
    const scanPayload = (await scanResponse.json()) as {
      escalated_runs?: Array<{ run_id: string; token_address: string; signal_tag: string; reason: string }>;
    };
    lastPayload = scanPayload;

    const escalatedRun = scanPayload.escalated_runs?.find(
      (run) => run.token_address === tokenAddress && run.signal_tag === signal
    );
    if (escalatedRun) {
      return escalatedRun;
    }

    await new Promise((resolve) => setTimeout(resolve, 750));
  }

  throw new Error(
    `Expected ${signal} escalation for ${tokenAddress} after ${maxAttempts} auto-scan attempts. Last payload: ${JSON.stringify(lastPayload)}`
  );
}

async function waitForAnyEscalatedRun(
  request: import('@playwright/test').APIRequestContext,
  tokenAddress: string,
  maxAttempts = 3
) {
  let lastPayload:
    | {
        escalated_runs?: Array<{ run_id: string; token_address: string; signal_tag: string }>;
      }
    | undefined;

  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    const scanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
    expect(scanResponse.ok()).toBeTruthy();
    const scanPayload = (await scanResponse.json()) as {
      escalated_runs?: Array<{ run_id: string; token_address: string; signal_tag: string }>;
    };
    lastPayload = scanPayload;

    const escalatedRun = scanPayload.escalated_runs?.find((run) => run.token_address === tokenAddress);
    if (escalatedRun) {
      return escalatedRun;
    }

    await new Promise((resolve) => setTimeout(resolve, 750));
  }

  throw new Error(
    `Expected any escalation for ${tokenAddress} after ${maxAttempts} auto-scan attempts. Last payload: ${JSON.stringify(lastPayload)}`
  );
}

test.describe('Runs inbox', () => {
  test('shows a persisted investigation run created through the live backend', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await page.goto('/mia/runs?status=completed&trigger=manual', { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('mia-runs-heading')).toHaveText('Active and recent investigations in one place');
    await expect(page.getByTestId('mia-runs-status-completed')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-runs-trigger-manual')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-runs-filter-summary')).toContainText('completed');
    await expect(page.getByTestId('mia-runs-filter-summary')).toContainText('manual');
    await expect(page.getByTestId('mia-runs-ops-summary')).toBeVisible();
    await expect(page.getByTestId('mia-runs-ops-run-states')).toContainText(/Queued|Watching|Escalated|Completed/i);
    await expect(page.getByTestId('mia-run-row').first()).toContainText(tokenAddress);
    await expect(page.getByRole('link', { name: 'Open Investigation' }).first()).toHaveAttribute(
      'href',
      new RegExp(`/mia\\?q=${encodeURIComponent(tokenAddress)}`)
    );
  });

  test('runs inbox can pause and resume auto investigation from the operator summary', async ({ page, request }) => {
    await page.goto('/mia/runs', { waitUntil: 'domcontentloaded' });

    const autoState = page.getByTestId('mia-runs-auto-state');
    await expect(autoState).toBeVisible();

    const initialText = (await autoState.textContent()) ?? '';
    const initiallyPaused = /paused/i.test(initialText);

    await page.getByTestId('mia-runs-toggle-auto-investigation').click();

    if (initiallyPaused) {
      await expect(autoState).toContainText(/live/i);
      await expect(page.getByTestId('mia-runs-control-notice')).toContainText(/resumed/i);
    } else {
      await expect(autoState).toContainText(/paused/i);
      await expect(page.getByTestId('mia-runs-control-notice')).toContainText(/paused/i);
    }

    const toggledScanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
    expect(toggledScanResponse.ok()).toBeTruthy();
    const toggledScanPayload = (await toggledScanResponse.json()) as { paused: boolean };
    expect(toggledScanPayload.paused).toBe(!initiallyPaused);

    await page.getByTestId('mia-runs-toggle-auto-investigation').click();
    if (initiallyPaused) {
      await expect(autoState).toContainText(/paused/i);
    } else {
      await expect(autoState).toContainText(/live/i);
    }

    const restoredScanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
    expect(restoredScanResponse.ok()).toBeTruthy();
    const restoredScanPayload = (await restoredScanResponse.json()) as { paused: boolean };
    expect(restoredScanPayload.paused).toBe(initiallyPaused);
  });

  test('runs inbox can archive terminal runs from the operator controls', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    const completedRunsResponse = await request.get(
      `/api/backend/api/v1/investigations/runs?limit=10&token=${encodeURIComponent(tokenAddress)}&status=completed&trigger=manual`
    );
    expect(completedRunsResponse.ok()).toBeTruthy();
    const completedRunsPayload = (await completedRunsResponse.json()) as {
      data: Array<{ run_id: string; status: string }>;
    };

    const targetRun = completedRunsPayload.data[0];
    expect(targetRun).toBeTruthy();
    expect(targetRun?.status).toBe('completed');

    await page.goto('/mia/runs?status=completed&trigger=manual', { waitUntil: 'domcontentloaded' });
    await page.getByTestId('mia-runs-archive-stale').click();

    await expect(page.getByTestId('mia-runs-control-notice')).toContainText(/archived/i);

    const archivedRunsResponse = await request.get(
      `/api/backend/api/v1/investigations/runs?limit=100&status=archived&trigger=manual`
    );
    expect(archivedRunsResponse.ok()).toBeTruthy();
    const archivedRunsPayload = (await archivedRunsResponse.json()) as {
      data: Array<{ run_id: string; status: string }>;
    };

    const archivedRun = archivedRunsPayload.data.find((run) => run.run_id === targetRun?.run_id);
    expect(archivedRun).toBeTruthy();
    expect(archivedRun?.status).toBe('archived');
  });

  test('runs inbox can retry failed runs from the operator controls', async ({ page, request }) => {
    const fixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/failed-run');
    expect(fixtureResponse.ok()).toBeTruthy();
    const fixturePayload = (await fixtureResponse.json()) as {
      token_address: string;
      run_id: string;
    };

    await page.goto('/mia/runs?status=failed&trigger=manual', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-runs-status-failed')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-run-row').filter({ hasText: fixturePayload.token_address }).first()).toBeVisible();

    await page.getByTestId('mia-runs-retry-failed').click();
    await expect(page.getByTestId('mia-runs-control-notice')).toContainText(/re-queued/i);

    const queuedRunsResponse = await request.get(
      `/api/backend/api/v1/investigations/runs?limit=20&token=${encodeURIComponent(fixturePayload.token_address)}&status=queued&trigger=manual`
    );
    expect(queuedRunsResponse.ok()).toBeTruthy();
    const queuedRunsPayload = (await queuedRunsResponse.json()) as {
      data: Array<{ run_id: string; status: string; current_stage: string }>;
    };

    const retriedRun = queuedRunsPayload.data.find((run) => run.run_id === fixturePayload.run_id);
    expect(retriedRun).toBeTruthy();
    expect(retriedRun?.status).toBe('queued');
    expect(retriedRun?.current_stage).toBe('retry_queued');
  });

  test('runs inbox can recover stale running runs from the operator controls', async ({ page, request }) => {
    const fixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/stale-running');
    expect(fixtureResponse.ok()).toBeTruthy();
    const fixturePayload = (await fixtureResponse.json()) as {
      token_address: string;
      run_id: string;
    };

    await page.goto('/mia/runs?status=running&trigger=manual', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-runs-status-running')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-run-row').filter({ hasText: fixturePayload.token_address }).first()).toBeVisible();

    await page.getByTestId('mia-runs-recover-stale').click();
    await expect(page.getByTestId('mia-runs-control-notice')).toContainText(/recovered/i);

    const queuedRunsResponse = await request.get(
      `/api/backend/api/v1/investigations/runs?limit=20&token=${encodeURIComponent(fixturePayload.token_address)}&status=queued&trigger=manual`
    );
    expect(queuedRunsResponse.ok()).toBeTruthy();
    const queuedRunsPayload = (await queuedRunsResponse.json()) as {
      data: Array<{ run_id: string; status: string; current_stage: string }>;
    };

    const recoveredRun = queuedRunsPayload.data.find((run) => run.run_id === fixturePayload.run_id);
    expect(recoveredRun).toBeTruthy();
    expect(recoveredRun?.status).toBe('queued');
    expect(recoveredRun?.current_stage).toBe('recovery_queued');
  });

  test('runs inbox exposes loop-health metrics after retry and recovery controls fire', async ({ page, request }) => {
    const failedFixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/failed-run');
    expect(failedFixtureResponse.ok()).toBeTruthy();

    const staleFixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/stale-running');
    expect(staleFixtureResponse.ok()).toBeTruthy();

    await page.goto('/mia/runs', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-runs-ops-loop-health')).toBeVisible();

    await page.getByTestId('mia-runs-retry-failed').click();
    await expect(page.getByTestId('mia-runs-control-notice')).toContainText(/re-queued/i);

    await page.getByTestId('mia-runs-recover-stale').click();
    await expect(page.getByTestId('mia-runs-control-notice')).toContainText(/recovered/i);

    const summaryResponse = await request.get('/api/backend/api/v1/investigations/ops/summary');
    expect(summaryResponse.ok()).toBeTruthy();
    const summaryPayload = (await summaryResponse.json()) as {
      loop_health: {
        auto_runs_24h: number;
        retry_actions_24h: number;
        recovery_actions_24h: number;
        failure_rate_24h_pct: number;
        average_completion_minutes_24h: number | null;
      };
    };

    expect(summaryPayload.loop_health.retry_actions_24h).toBeGreaterThanOrEqual(1);
    expect(summaryPayload.loop_health.recovery_actions_24h).toBeGreaterThanOrEqual(1);

    const loopHealthCard = page.getByTestId('mia-runs-ops-loop-health');
    await expect(loopHealthCard).toContainText(/Retries 24h/i);
    await expect(loopHealthCard).toContainText(/Recoveries 24h/i);
    await expect(loopHealthCard).toContainText(/Failure rate 24h/i);
    await expect(loopHealthCard).toContainText(/Avg completion/i);
  });

  test('runs inbox surfaces degradation notes when source-backed monitoring weakens', async ({ page, request }) => {
    const fixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/monitoring-downgrade', {
      data: { signal: 'source_degradation' },
    });
    expect(fixtureResponse.ok()).toBeTruthy();

    const scanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
    expect(scanResponse.ok()).toBeTruthy();

    await page.goto('/mia/runs', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-runs-degradation-note').filter({ hasText: /source health degraded/i }).first()).toBeVisible();
  });

  test('runs inbox surfaces report evidence-gap notes when latest citations are empty', async ({ page, request }) => {
    const fixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/monitoring-downgrade', {
      data: { signal: 'source_degradation' },
    });
    expect(fixtureResponse.ok()).toBeTruthy();

    await page.goto('/mia/runs', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-runs-degradation-note').filter({ hasText: /empty citations/i }).first()).toBeVisible();
  });

  test('investigation workspace exposes the active run above the fold', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await Promise.all([
      page.waitForResponse((response) => response.url().includes(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`) && response.ok()),
      page.goto(`/mia?q=${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' }),
    ]);

    await expect(page.getByTestId('mia-active-run-panel')).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId('mia-active-run-status')).toContainText(/completed/i);
    await expect(page.getByTestId('mia-open-runs-inbox')).toHaveAttribute(
      'href',
      /\/mia\/runs\?status=completed&trigger=manual/
    );
    await expect(page.getByTestId('mia-open-token-history')).toHaveAttribute(
      'href',
      new RegExp(`/mia/token/${encodeURIComponent(tokenAddress)}`)
    );
  });

  test('token history page shows the persisted run history for one token', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await page.goto(`/mia/token/${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('mia-token-history-heading')).toContainText('Token investigation history');
    await expect(page.getByTestId('mia-token-history-address')).toContainText(tokenAddress);
    await expect(page.getByTestId('mia-token-history-summary')).toContainText('manual');
    await expect(page.getByTestId('mia-token-history-row').first()).toContainText(tokenAddress);
    await expect(page.getByRole('link', { name: 'Open Investigation' }).first()).toHaveAttribute(
      'href',
      new RegExp(`/mia\\?q=${encodeURIComponent(tokenAddress)}`)
    );
  });

  test('run detail page shows the lightweight timeline for one persisted run', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    const runsResponse = await request.get(`/api/backend/api/v1/investigations/runs?limit=1&token=${encodeURIComponent(tokenAddress)}`);
    expect(runsResponse.ok()).toBeTruthy();
    const runsPayload = (await runsResponse.json()) as {
      data: Array<{ run_id: string }>;
    };

    expect(runsPayload.data.length).toBeGreaterThan(0);
    const runId = runsPayload.data[0]!.run_id;

    await page.goto(`/mia/runs/${encodeURIComponent(runId)}`, { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('mia-run-detail-heading')).toContainText('Investigation run detail');
    await expect(page.getByTestId('mia-run-detail-id')).toContainText(runId);
    await expect(page.getByTestId('mia-run-detail-status')).toContainText(/completed/i);
    await expect(page.getByTestId('mia-run-timeline-event').first()).toBeVisible();
    await expect(page.getByRole('link', { name: 'Open Token History' })).toHaveAttribute(
      'href',
      new RegExp(`/mia/token/${encodeURIComponent(tokenAddress)}`)
    );
  });

  test('investigation workspace exposes a timeline lane and run change summary', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await page.goto(`/mia?q=${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('mia-run-console-summary')).toBeVisible();
    await expect(page.getByTestId('mia-workspace-tab-timeline')).toHaveAttribute('aria-pressed', 'false');

    await page.getByTestId('mia-workspace-tab-timeline').click();

    await expect(page.getByTestId('mia-workspace-tab-timeline')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-run-console-change-note')).toBeVisible();
    await expect(page.getByTestId('mia-run-console-timeline-event').first()).toBeVisible();
  });

  test('investigation workspace can change the active run state and the inbox reflects it', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await page.goto(`/mia?q=${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' });

    await page.getByTestId('mia-run-action-watching').click();
    await expect(page.getByTestId('mia-active-run-status')).toContainText(/watching/i);
    await expect(page.getByTestId('mia-active-run-panel').getByText(/Monitoring reason:/)).toBeVisible();
    await expect(page.getByTestId('mia-active-run-panel').getByText(/Latest evidence delta:/)).toBeVisible();

    await page.getByTestId('mia-run-action-archived').click();
    await expect(page.getByTestId('mia-active-run-status')).toContainText(/archived/i);
    await expect(page.getByTestId('mia-active-run-panel').getByText(/Archive reason:/)).toBeVisible();
    await expect(page.getByTestId('mia-active-run-panel').getByText(/before archive/i)).toBeVisible();

    await page.goto('/mia/runs?status=archived&trigger=manual', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-runs-status-archived')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-run-row').filter({ hasText: tokenAddress }).first()).toBeVisible();
  });

  test('run detail keeps append-only state-transition history instead of only the latest state snapshot', async ({ page, request }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await Promise.all([
      page.waitForResponse((response) => response.url().includes(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`) && response.ok()),
      page.goto(`/mia?q=${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' }),
    ]);

    const runDetailHref = await page.getByTestId('mia-open-run-detail').getAttribute('href');
    expect(runDetailHref).toBeTruthy();

    await page.goto(runDetailHref!, { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-run-timeline-event').first()).toBeVisible();
    const statusChangeEvents = page.getByTestId('mia-run-timeline-event').filter({ hasText: 'Run status change' });
    const initialStatusChangeCount = await statusChangeEvents.count();

    await page.goto(`/mia?q=${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' });

    await page.getByTestId('mia-run-action-watching').click();
    await expect(page.getByTestId('mia-active-run-status')).toContainText(/watching/i);

    await page.getByTestId('mia-run-action-archived').click();
    await expect(page.getByTestId('mia-active-run-status')).toContainText(/archived/i);

    await page.goto(runDetailHref!, { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('mia-run-timeline-event').first()).toBeVisible();
    await expect
      .poll(async () => page.getByTestId('mia-run-timeline-event').filter({ hasText: 'Run status change' }).count())
      .toBeGreaterThanOrEqual(initialStatusChangeCount + 2);
    await expect(page.getByTestId('mia-run-timeline-event').filter({ hasText: /Monitoring reason:/ }).first()).toBeVisible();
    await expect(page.getByTestId('mia-run-timeline-event').filter({ hasText: /Archive reason:/ }).first()).toBeVisible();
  });

  test('auto investigation scan queues system-started runs and surfaces them in the inbox', async ({ page, request }) => {
    const scanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
    expect(scanResponse.ok()).toBeTruthy();
    const scanPayload = (await scanResponse.json()) as {
      matched_candidates: number;
      created_runs: Array<{ token_address: string }>;
      skipped_tokens: Array<{ token_address: string }>;
    };

    expect(scanPayload.matched_candidates).toBeGreaterThan(0);
    const createdTokenAddress = scanPayload.created_runs[0]?.token_address ?? null;

    await page.goto('/mia/runs?trigger=auto', { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('mia-runs-trigger-auto')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-runs-filter-summary')).toContainText('auto');
    await expect(page.getByTestId('mia-run-row').first()).toContainText(/auto/i);
    if (createdTokenAddress) {
      await expect(
        page.getByTestId('mia-run-row').filter({ hasText: createdTokenAddress }).first()
      ).toBeVisible();
    }
  });

  test('auto investigation scan can escalate an existing watching run', async ({ page, request }) => {
    const bootstrapScanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
    expect(bootstrapScanResponse.ok()).toBeTruthy();
    const bootstrapPayload = (await bootstrapScanResponse.json()) as {
      created_runs: Array<{ token_address: string }>;
      skipped_tokens: Array<{ token_address: string }>;
    };

    const tokenAddress =
      bootstrapPayload.created_runs[0]?.token_address ?? bootstrapPayload.skipped_tokens[0]?.token_address;
    expect(tokenAddress).toBeTruthy();

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await Promise.all([
      page.waitForResponse((response) => response.url().includes(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`) && response.ok()),
      page.goto(`/mia?q=${encodeURIComponent(tokenAddress!)}`, { waitUntil: 'domcontentloaded' }),
    ]);

    await page.getByTestId('mia-run-action-watching').click();
    await expect(page.getByTestId('mia-active-run-status')).toContainText(/watching/i);

    const scanEscalatedRun = await waitForAnyEscalatedRun(request, tokenAddress!);
    expect(scanEscalatedRun).toBeTruthy();

    const runsResponse = await request.get(`/api/backend/api/v1/investigations/runs?limit=10&token=${encodeURIComponent(tokenAddress!)}`);
    expect(runsResponse.ok()).toBeTruthy();
    const runsPayload = (await runsResponse.json()) as {
      data: Array<{ run_id: string; status: string; status_reason: string | null }>;
    };

    const escalatedRun = runsPayload.data.find((run) => run.status === 'escalated');
    expect(escalatedRun).toBeTruthy();
    expect(escalatedRun?.status_reason).toMatch(/Auto escalation reason:/);

    await page.goto(`/mia/runs/${encodeURIComponent(escalatedRun!.run_id)}`, { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-run-detail-status')).toContainText(/escalated/i);
    const expectedBadge =
      scanEscalatedRun?.signal_tag === 'wallet_concentration'
        ? /wallet concentration/i
        : scanEscalatedRun?.signal_tag === 'whale_alert'
          ? /whale alert/i
          : scanEscalatedRun?.signal_tag === 'builder_overlap'
            ? /builder overlap/i
            : scanEscalatedRun?.signal_tag === 'linked_launch_overlap'
              ? /linked launch overlap/i
          : scanEscalatedRun?.signal_tag === 'multi_signal'
            ? /multi-signal/i
            : /activity/i;
    await expect(page.getByTestId('mia-run-detail-signal-badge')).toContainText(expectedBadge);
    await expect(page.getByTestId('mia-run-detail-timeline-signal-badge').first()).toContainText(expectedBadge);
  });

  for (const scenario of nonTransactionEscalationScenarios) {
    test(`auto investigation scan can escalate a ${scenario.signal} fixture without crossing the tx threshold`, async ({
      page,
      request,
    }) => {
      const fixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/non-tx-escalation', {
        data: { signal: scenario.signal },
      });
      expect(fixtureResponse.ok()).toBeTruthy();
      const fixturePayload = (await fixtureResponse.json()) as {
        token_address: string;
        run_id: string;
        signal_tag: string;
        tx_count: number;
      };

      expect(fixturePayload.signal_tag).toBe(scenario.signal);
      expect(fixturePayload.tx_count).toBeLessThan(100);

      const escalatedRun = await waitForEscalatedRun(request, fixturePayload.token_address, scenario.signal);
      expect(escalatedRun?.reason).toMatch(scenario.reason);

      const runsResponse = await request.get(
        `/api/backend/api/v1/investigations/runs?limit=10&token=${encodeURIComponent(fixturePayload.token_address)}`
      );
      expect(runsResponse.ok()).toBeTruthy();
      const runsPayload = (await runsResponse.json()) as {
        data: Array<{ run_id: string; status: string; status_reason: string | null }>;
      };

      const persistedRun = runsPayload.data.find((run) => run.run_id === fixturePayload.run_id);
      expect(persistedRun).toBeTruthy();
      expect(persistedRun?.status).toBe('escalated');
      expect(persistedRun?.status_reason).toMatch(scenario.reason);

      await page.goto(`/mia/runs/${encodeURIComponent(fixturePayload.run_id)}`, { waitUntil: 'domcontentloaded' });
      await expect(page.getByTestId('mia-run-detail-status')).toContainText(/escalated/i);
      await expect(page.getByTestId('mia-run-detail-signal-badge')).toContainText(scenario.badge);
      await expect(page.getByTestId('mia-run-detail-timeline-signal-badge').first()).toContainText(scenario.badge);
    });
  }

  for (const scenario of monitoringDowngradeScenarios) {
    test(`auto investigation scan can downgrade a stale ${scenario.signal} fixture back to watching`, async ({
      page,
      request,
    }) => {
      const fixtureResponse = await request.post('/api/backend/api/v1/investigations/test-fixtures/monitoring-downgrade', {
        data: { signal: scenario.signal },
      });
      expect(fixtureResponse.ok()).toBeTruthy();
      const fixturePayload = (await fixtureResponse.json()) as {
        token_address: string;
        run_id: string;
        signal_tag: string;
        tx_count: number;
      };

      expect(fixturePayload.signal_tag).toBe(scenario.signal);
      expect(fixturePayload.tx_count).toBeLessThan(100);

      const scanResponse = await request.post('/api/backend/api/v1/investigations/auto-scan');
      expect(scanResponse.ok()).toBeTruthy();
      const scanPayload = (await scanResponse.json()) as {
        downgraded_runs?: Array<{ run_id: string; token_address: string; signal_tag?: string | null; reason: string }>;
      };

      const downgradedRun = scanPayload.downgraded_runs?.find(
        (run) =>
          run.token_address === fixturePayload.token_address &&
          run.run_id === fixturePayload.run_id &&
          run.signal_tag === scenario.signal
      );
      expect(downgradedRun).toBeTruthy();
      expect(downgradedRun?.reason).toMatch(scenario.reason);

      const runsResponse = await request.get(
        `/api/backend/api/v1/investigations/runs?limit=10&token=${encodeURIComponent(fixturePayload.token_address)}`
      );
      expect(runsResponse.ok()).toBeTruthy();
      const runsPayload = (await runsResponse.json()) as {
        data: Array<{ run_id: string; status: string; status_reason: string | null }>;
      };

      const persistedRun = runsPayload.data.find((run) => run.run_id === fixturePayload.run_id);
      expect(persistedRun).toBeTruthy();
      expect(persistedRun?.status).toBe('watching');
      expect(persistedRun?.status_reason).toMatch(scenario.reason);

      await page.goto(`/mia/runs/${encodeURIComponent(fixturePayload.run_id)}`, { waitUntil: 'domcontentloaded' });
      await expect(page.getByTestId('mia-run-detail-status')).toContainText(/watching/i);
      await expect(page.getByTestId('mia-run-detail-signal-badge')).toContainText(scenario.badge);
      await expect(page.getByTestId('mia-run-timeline-event').filter({ hasText: /Auto monitoring downgrade/i }).first()).toBeVisible();
      await expect(page.getByTestId('mia-run-timeline-event').filter({ hasText: scenario.reason }).first()).toBeVisible();
    });
  }
});
