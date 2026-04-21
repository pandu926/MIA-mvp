import { expect, test } from '@playwright/test';

test.describe('Supporting surfaces clarity', () => {
  test('tokens and alpha pages are framed as supporting surfaces and route back into the main flow', async ({ page }) => {
    await page.goto('/tokens', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('tokens-page-heading')).toHaveText('Structured token index for deeper filtering');
    await expect(page.getByTestId('tokens-open-discover')).toHaveText('Open Discover');

    await page.goto('/alpha', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('alpha-page-heading')).toHaveText('Alpha ranking for fast prioritization');
    await expect(page.getByTestId('alpha-open-discover')).toHaveText('Open Discover');

    const featuredInvestigationCta = page.getByTestId('alpha-featured-open-investigation');
    if ((await featuredInvestigationCta.count()) > 0) {
      await expect(featuredInvestigationCta).toHaveAttribute('href', /\/mia\?q=/);
    } else {
      const alphaFallback = page
        .getByText('Loading alpha feed...')
        .or(page.getByText('Waiting for ranking window'))
        .or(page.getByText(/Failed to load alpha rankings/i));
      await expect(alphaFallback.first()).toBeVisible();
    }
  });
});
