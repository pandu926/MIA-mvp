import { expect, test } from '@playwright/test';

const TOKEN_ADDRESS = '0x6b5901ddf24fc212de3c59a7a078df4a67974444';
const TOKEN_LABEL = 'Why is this risky?';

test.describe('Ask MIA v2', () => {
  test('opens chat mode and renders a function-calling answer', async ({ page, request }) => {
    const investigationResponse = await request.get(`/api/backend/api/v1/tokens/${TOKEN_ADDRESS}/investigation`);
    expect(investigationResponse.ok()).toBeTruthy();
    const investigationPayload = (await investigationResponse.json()) as {
      active_run: { run_id: string } | null;
    };
    expect(investigationPayload.active_run).not.toBeNull();

    await page.goto(`/mia/ask?q=${TOKEN_ADDRESS}&label=${encodeURIComponent(TOKEN_LABEL)}&run=${encodeURIComponent(investigationPayload.active_run!.run_id)}`, {
      waitUntil: 'domcontentloaded',
    });
    await expect(
      page.getByRole('heading', { name: /ask direct questions about/i })
    ).toBeVisible({ timeout: 20000 });
    await expect(page.getByTestId('ask-mia-run-context')).toBeVisible();

    await page
      .getByRole('button', { name: 'Why is this risky?' })
      .click();

    const response = page.getByTestId('ask-mia-chat-response');
    await expect(response.getByText('Short answer')).toBeVisible();
    await expect(response.getByText('function calling')).toBeVisible();
    await expect(response.getByTestId('ask-mia-chat-run-aware')).toBeVisible();
    await expect(response.getByText('Attached run context')).toBeVisible();
    await expect(response.getByText('gpt-4o-mini')).toBeVisible();
    await expect(page.getByTestId('ask-mia-chat-short-answer')).not.toBeEmpty();
    await expect(response.getByText('Why MIA says this')).toBeVisible();
    await expect(response.getByText('What to do next')).toBeVisible();
    const toolActivity = page.getByTestId('ask-mia-chat-tool-activity');
    await expect(toolActivity).toBeVisible();

    const toolActivityText = (await toolActivity.textContent()) ?? '';
    expect(toolActivityText).toMatch(
      /Token overview|Risk snapshot|Decision scorecard|Market structure|Wallet structure|Operator pattern|Builder memory|Whale and flow signals|ML context|Narrative context/
    );

    const html = await page.content();
    expect(html).toContain(TOKEN_LABEL);
  });
});
