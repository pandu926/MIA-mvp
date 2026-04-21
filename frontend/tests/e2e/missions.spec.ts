import { expect, test } from '@playwright/test';

test.describe('Missions', () => {
  test('watchlist context can create and operate a persistent mission', async ({ page, request }) => {
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
    await expect(page.getByTestId('mia-active-run-panel')).toBeVisible();

    await page.getByTestId('mia-save-token-watch').click();
    await expect(page.getByTestId('mia-watch-save-notice')).toContainText(/watch saved/i);

    await page.goto('/mia/watchlist', { waitUntil: 'domcontentloaded' });
    const tokenRow = page.getByTestId('mia-watchlist-row').filter({ hasText: tokenAddress }).first();
    await expect(tokenRow).toBeVisible();
    await tokenRow.getByTestId('mia-watchlist-open-missions').click();

    await expect(page.getByTestId('mia-missions-heading')).toContainText('Operate persistent objectives without losing context');
    await expect(page.getByTestId('mia-missions-context')).toContainText(tokenAddress);

    await page.getByTestId('mia-mission-template-watch_hot_launches').click();
    await expect(page.getByTestId('mia-missions-notice')).toContainText(/Mission created/i);

    const missionsResponse = await request.get('/api/backend/api/v1/investigations/missions');
    expect(missionsResponse.ok()).toBeTruthy();
    const missionsPayload = (await missionsResponse.json()) as {
      data: Array<{ mission_id: string; entity_key: string | null; mission_type: string }>;
    };
    const createdMission = missionsPayload.data.find(
      (mission) => mission.entity_key === tokenAddress && mission.mission_type === 'watch_hot_launches'
    );
    expect(createdMission).toBeTruthy();

    const missionRow = page.locator(`[data-mission-id="${createdMission!.mission_id}"]`);
    await expect(missionRow).toBeVisible();
    await expect(missionRow).toContainText(/Watch hot launches/i);

    await missionRow.getByTestId('mia-mission-pause').click();
    await expect(page.getByTestId('mia-missions-notice')).toContainText(/paused/i);
    await expect(missionRow).toContainText(/paused/i);

    await missionRow.getByTestId('mia-mission-resume').click();
    await expect(page.getByTestId('mia-missions-notice')).toContainText(/active/i);
    await expect(missionRow).toContainText(/active/i);
  });
});
