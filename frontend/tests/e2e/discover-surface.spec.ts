import { expect, test } from '@playwright/test';

test.describe('Discover surface', () => {
  test('discover shows live summary and routes urgent work into investigation or runs', async ({ page }) => {
    await page.goto('/app', { waitUntil: 'domcontentloaded' });

    await expect(page.getByTestId('discover-page-heading')).toHaveText('Find live launches worth investigating.');
    await expect(page.getByTestId('discover-live-summary')).toBeVisible();
    await expect(page.getByTestId('discover-summary-active-runs')).toBeVisible();
    await expect(page.getByTestId('discover-summary-watch-items')).toBeVisible();
    await expect(page.getByTestId('discover-needs-attention')).toBeVisible();
    await expect(page.getByTestId('discover-open-investigation')).toBeVisible();
    await expect(page.getByTestId('discover-open-runs')).toBeVisible();

    const priorityRows = page.getByTestId('discover-priority-run-row');
    const priorityCount = await priorityRows.count();

    if (priorityCount > 0) {
      await expect(priorityRows.first()).toBeVisible();
      await expect(page.getByTestId('discover-priority-open-investigation').first()).toBeVisible();
    } else {
      await expect(page.getByTestId('discover-needs-attention')).toContainText(/No escalated or watching runs are active right now/i);
    }
  });
});
