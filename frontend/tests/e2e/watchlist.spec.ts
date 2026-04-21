import { expect, test } from '@playwright/test';

test.describe('Watchlist', () => {
  test('investigation workspace can save token and builder watches into the persistent watchlist', async ({
    page,
    request,
  }) => {
    const tokenListResponse = await request.get('/api/backend/api/v1/tokens?limit=1');
    expect(tokenListResponse.ok()).toBeTruthy();
    const tokenList = (await tokenListResponse.json()) as {
      data: Array<{ contract_address: string; deployer_address: string }>;
    };

    expect(tokenList.data.length).toBeGreaterThan(0);
    const tokenAddress = tokenList.data[0]!.contract_address;
    const deployerAddress = tokenList.data[0]!.deployer_address;

    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();

    await Promise.all([
      page.waitForResponse(
        (response) =>
          response.url().includes(`/api/backend/api/v1/tokens/${tokenAddress}/investigation`) && response.ok()
      ),
      page.goto(`/mia?q=${encodeURIComponent(tokenAddress)}`, { waitUntil: 'domcontentloaded' }),
    ]);
    await expect(page.getByTestId('mia-active-run-panel')).toBeVisible();

    await page.getByTestId('mia-save-token-watch').click();
    await expect(page.getByTestId('mia-watch-save-notice')).toContainText(/watch saved/i);

    await page.getByTestId('mia-save-builder-watch').click();
    await expect(page.getByTestId('mia-watch-save-notice')).toContainText(/watch saved/i);

    await page.goto('/mia/watchlist', { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('mia-watchlist-heading')).toContainText('Monitor saved tokens and builders with continuity');
    await expect(page.getByTestId('mia-watchlist-row').filter({ hasText: tokenAddress }).first()).toBeVisible();
    await expect(page.getByTestId('mia-watchlist-row').filter({ hasText: deployerAddress }).first()).toBeVisible();
    await expect(page.getByTestId('mia-watchlist-row').filter({ hasText: tokenAddress }).getByText(/Open Investigation/i)).toBeVisible();
  });
});
