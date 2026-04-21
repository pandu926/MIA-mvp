import { expect, test } from '@playwright/test';

test.describe('Investigation workspace redesign', () => {
  test('mia workspace stays readable on mobile while preserving run actions and drill-ins', async ({
    page,
    request,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });

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
      page.waitForResponse(
        (response) =>
          response.url().includes(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`) && response.ok()
      ),
      page.goto(`/mia?q=${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' }),
    ]);

    await expect(page.getByTestId('mia-page-heading')).toBeVisible();
    await expect(page.getByTestId('mia-intent-quick')).toBeVisible();
    await expect(page.getByTestId('mia-intent-deep')).toBeVisible();
    await expect(page.getByTestId('mia-mobile-hero-card')).toBeVisible();
    await expect(page.getByTestId('mia-evidence-signals-card')).toBeVisible();
    await expect(page.getByTestId('mia-top-evidence-strip')).toBeVisible();
    await expect(page.getByTestId('mia-active-run-panel')).toBeVisible();
    await expect(page.getByTestId('mia-run-action-watching')).toBeVisible();
    await expect(page.getByTestId('mia-open-runs-inbox')).toBeVisible();
    await expect(page.getByTestId('mia-run-console-summary')).toBeVisible();
    await expect(page.getByTestId('mia-workspace-tab-timeline')).toBeVisible();

    await page.getByTestId('mia-intent-deep').click();
    await expect(page.getByTestId('mia-workspace-tab-tools')).toHaveAttribute('aria-pressed', 'true');

    await page.getByTestId('mia-workspace-tab-timeline').click();
    await expect(page.getByTestId('mia-workspace-tab-timeline')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('mia-run-console-timeline')).toBeVisible();
  });
});
