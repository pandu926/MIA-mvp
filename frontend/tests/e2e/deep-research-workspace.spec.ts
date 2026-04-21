import { expect, test } from '@playwright/test';

const TOKEN_ADDRESS = '0x6b5901ddf24fc212de3c59a7a078df4a67974444';
const RUN_ID = '0f1b9803-4f3c-42b7-8bc1-fcb5759d4d53';

test.describe('Deep Research workspace', () => {
  test('starts a run and renders the phase 3 trace workspace', async ({ page }) => {
    await page.addInitScript((address) => {
      window.localStorage.setItem(
        `mia:deep-research-entitlement:${address.toLowerCase()}`,
        'entitled-token'
      );
    }, TOKEN_ADDRESS);

    await page.route(new RegExp(`/api/backend/api/v1/tokens/${TOKEN_ADDRESS}/deep-research/status`), async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          token_address: TOKEN_ADDRESS,
          premium_state: 'report_ready',
          provider_path: 'MIA launch intelligence + optional narrative enrichment',
          unlock_model: 'x402',
          x402_enabled: true,
          report_cached: true,
          has_active_entitlement: true,
          entitlement_expires_at: null,
          native_x_api_reserved: true,
          notes: [],
        }),
      });
    });

    await page.route(new RegExp(`/api/backend/api/v1/tokens/${TOKEN_ADDRESS}/deep-research/preview`), async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          token_address: TOKEN_ADDRESS,
          enabled: true,
          provider_path: 'MIA launch intelligence + optional narrative enrichment',
          unlock_model: 'x402',
          unlock_cta: 'Unlock Deep Research',
          payment_network: 'base',
          price_usdc_cents: 50,
          sections: [
            {
              id: 'dex-market',
              title: 'Dex market structure',
              summary: 'Attach pair quality and market structure.',
              stage: 'mvp',
            },
          ],
          sybil_policy: {
            wording: 'pattern warning',
            confidence_model: 'signal-based',
            promise: 'no identity claims',
          },
          notes: [],
        }),
      });
    });

    await page.route(new RegExp(`/api/backend/api/v1/tokens/${TOKEN_ADDRESS}/deep-research$`), async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          token_address: TOKEN_ADDRESS,
          provider_path: 'MIA launch intelligence + optional narrative enrichment',
          status: 'ready',
          executive_summary: 'Cached premium report is ready.',
          sections: [
            {
              id: 'wallet-structure',
              title: 'Wallet structure',
              summary: 'Wallet concentration is elevated.',
              stage: 'mvp',
              provider: 'mia_internal',
              confidence: 'high',
              evidence: ['Repeated wallets were recovered.'],
            },
          ],
          citations: [],
          source_status: {},
          generated_at: '2026-04-19T10:00:03Z',
          entitlement: null,
        }),
        headers: {
          'x-mia-entitlement': 'entitled-token',
        },
      });
    });

    await page.route(new RegExp(`/api/backend/api/v1/tokens/${TOKEN_ADDRESS}/deep-research/runs$`), async (route) => {
      if (route.request().method() !== 'POST') {
        await route.continue();
        return;
      }

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          run_id: RUN_ID,
          token_address: TOKEN_ADDRESS,
          provider_path: 'MIA launch intelligence + optional narrative enrichment',
          status: 'completed',
          current_phase: 'finalize',
          budget_usage_cents: 0,
          paid_calls_count: 0,
          report_ready: true,
          error_message: null,
          created_at: '2026-04-19T10:00:00Z',
          started_at: '2026-04-19T10:00:01Z',
          completed_at: '2026-04-19T10:00:03Z',
        }),
      });
    });

    await page.route(new RegExp(`/api/backend/api/v1/tokens/${TOKEN_ADDRESS}/deep-research/runs/${RUN_ID}/trace$`), async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          run_id: RUN_ID,
          token_address: TOKEN_ADDRESS,
          provider_path: 'MIA launch intelligence + optional narrative enrichment',
          status: 'completed',
          current_phase: 'finalize',
          budget_usage_cents: 0,
          paid_calls_count: 0,
          error_message: null,
          created_at: '2026-04-19T10:00:00Z',
          started_at: '2026-04-19T10:00:01Z',
          completed_at: '2026-04-19T10:00:03Z',
          steps: [
            {
              id: 1,
              step_key: 'plan',
              title: 'Plan',
              status: 'completed',
              agent_name: 'PlannerAgent',
              tool_name: 'get_market_structure, get_wallet_structure',
              summary: 'Planner created a stable internal research plan.',
              evidence: ['Planned steps: 5.', 'Required tools: 4.'],
              cost_cents: 0,
              payment_tx: null,
              started_at: '2026-04-19T10:00:01Z',
              completed_at: '2026-04-19T10:00:03Z',
            },
          ],
          tool_calls: [
            {
              id: 1,
              step_key: 'gather_internal',
              tool_name: 'get_market_structure',
              provider: 'mia_internal',
              status: 'completed',
              summary: 'Market structure attached.',
              evidence: ['Liquidity quality is stable.'],
              latency_ms: 85,
              cost_cents: 0,
              payment_tx: null,
              created_at: '2026-04-19T10:00:01Z',
              completed_at: '2026-04-19T10:00:03Z',
            },
          ],
          payment_ledger: [
            {
              id: 1,
              tool_call_id: 1,
              provider: 'heurist_mesh_x402',
              network: 'base',
              asset: 'USDC',
              amount_units: '2000',
              amount_display: '0.002000 USDC',
              tx_hash: '0xa34b7ab7843b63decac5e3134402023d488978f17a6ac8779df51b2ccf6ce607',
              status: 'completed',
              created_at: '2026-04-19T10:00:03Z',
            },
          ],
        }),
      });
    });

    await page.route(new RegExp(`/api/backend/api/v1/tokens/${TOKEN_ADDRESS}/deep-research/runs/${RUN_ID}/report$`), async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          token_address: TOKEN_ADDRESS,
          provider_path: 'MIA launch intelligence + optional narrative enrichment',
          status: 'ready',
          executive_summary: 'Premium dossier assembled successfully.',
          sections: [
            {
              id: 'wallet-structure',
              title: 'Wallet structure',
              summary: 'Wallet concentration is elevated.',
              stage: 'mvp',
              provider: 'mia_internal',
              confidence: 'high',
              evidence: ['Repeated wallets were recovered.'],
            },
          ],
          citations: [],
          source_status: {},
          generated_at: '2026-04-19T10:00:03Z',
          entitlement: null,
        }),
      });
    });

    await page.goto(`/mia?q=${TOKEN_ADDRESS}`, { waitUntil: 'domcontentloaded' });

    await expect(page.getByText('Investigation Score', { exact: true }).first()).toBeVisible({
      timeout: 30000,
    });
    await page.getByTestId('mia-workspace-tab-tools').click();
    await expect(page.getByText('Optional Monitoring and Route Tools', { exact: true })).toBeVisible();
    await expect(page.getByTestId('deep-research-start-run')).toBeVisible({ timeout: 20000 });
    await page.getByTestId('deep-research-start-run').click();

    await expect(page.getByTestId('deep-research-run-header')).toBeVisible();
    await expect(page.getByTestId('deep-research-run-trace')).toContainText(
      'Planner created a stable internal research plan.'
    );
    await expect(page.getByTestId('deep-research-tool-ledger')).toContainText(
      'get market structure'
    );
    await expect(page.getByText('Premium dossier assembled successfully.')).toBeVisible();
  });
});
