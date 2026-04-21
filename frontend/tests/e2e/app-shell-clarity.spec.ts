import { expect, test } from '@playwright/test';

test.describe('App shell clarity', () => {
  test('primary app shell locks to discover, runs, watch, missions, and proof', async ({ page }) => {
    await page.goto('/app', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('discover-page-heading')).toHaveText('Find live launches worth investigating.');
    await expect(page.getByTestId('discover-open-investigation')).toHaveText('Open Investigation');
    await expect(page.getByTestId('discover-open-runs')).toHaveText('Open Runs');
    await expect(page.getByTestId('obsidian-nav-link-discover')).toBeVisible();
    await expect(page.getByTestId('obsidian-nav-link-runs')).toBeVisible();
    await expect(page.getByTestId('obsidian-nav-link-watch')).toBeVisible();
    await expect(page.getByTestId('obsidian-nav-link-missions')).toBeVisible();
    await expect(page.getByTestId('obsidian-nav-link-proof')).toBeVisible();
    await expect(page.getByTestId('obsidian-nav-link-investigation')).toHaveCount(0);
    await expect(page.getByTestId('obsidian-nav-link-watchlist')).toHaveCount(0);

    await page.goto('/mia', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-page-heading')).toHaveText('Open a token investigation and inspect the evidence.');
    await expect(page.getByTestId('obsidian-nav-primary-action')).toHaveText(/Open Runs/i);

    await page.goto('/mia/runs', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-runs-heading')).toHaveText('Operate live investigations from one console');
    await expect(page.getByTestId('obsidian-nav-primary-action')).toHaveText(/Open Investigation/i);

    await page.goto('/mia/watchlist', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-watchlist-heading')).toHaveText('Monitor saved tokens and builders with continuity');

    await page.goto('/mia/missions', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('mia-missions-heading')).toHaveText('Operate persistent objectives without losing context');

    await page.goto('/backtesting', { waitUntil: 'domcontentloaded' });
    await expect(page.getByTestId('proof-page-heading')).toHaveText('Validate signal quality with replay and proof.');
    await expect(page.getByTestId('obsidian-nav-primary-action')).toHaveText(/Open Investigation/i);
  });
});
