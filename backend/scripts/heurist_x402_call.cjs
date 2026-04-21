#!/usr/bin/env node

const path = require('path');

const { wrapFetchWithPayment, x402Client } = require('@x402/fetch');
const { ExactEvmSchemeV1 } = require(path.join(
  __dirname,
  '..',
  '..',
  'node_modules',
  '@x402',
  'evm',
  'dist',
  'cjs',
  'v1',
  'index.js'
));
const { resolveWallet } = require('@bankofai/agent-wallet');
const BASE_USDC_ADDRESS = '0x833589fcdd6edb6e08f4c7c32d4f71b54bda02913';

function fail(message, details) {
  const payload = {
    ok: false,
    error: message,
    details: details ?? null,
  };
  process.stderr.write(`${JSON.stringify(payload)}\n`);
  process.exit(1);
}

async function main() {
  const [agentId, toolName, payloadJson] = process.argv.slice(2);
  if (!agentId || !toolName || !payloadJson) {
    fail('Usage: heurist_x402_call.cjs <agent_id> <tool_name> <payload_json>');
  }

  const walletDir = process.env.HEURIST_X402_WALLET_DIR || process.env.AGENT_WALLET_DIR;
  const walletPassword =
    process.env.HEURIST_X402_WALLET_PASSWORD || process.env.AGENT_WALLET_PASSWORD;
  const walletId = process.env.HEURIST_X402_WALLET_ID || 'mia-base-upstream';
  const rawBaseUrl = (process.env.HEURIST_X402_BASE_URL || 'https://mesh.heurist.xyz').replace(
    /\/$/,
    ''
  );
  const baseUrl = rawBaseUrl.endsWith('/x402')
    ? rawBaseUrl.slice(0, -'/x402'.length)
    : rawBaseUrl;

  if (!walletDir) {
    fail('HEURIST_X402_WALLET_DIR or AGENT_WALLET_DIR must be set.');
  }
  if (!walletPassword) {
    fail('HEURIST_X402_WALLET_PASSWORD or AGENT_WALLET_PASSWORD must be set.');
  }

  let payload;
  try {
    payload = JSON.parse(payloadJson);
  } catch (error) {
    fail('Payload JSON is invalid.', error instanceof Error ? error.message : String(error));
  }

  process.env.AGENT_WALLET_DIR = walletDir;
  process.env.AGENT_WALLET_PASSWORD = walletPassword;

  const wallet = await resolveWallet({
    network: 'eip155',
    walletId,
    dir: walletDir,
  });
  const payer = await wallet.getAddress();

  const signer = {
    address: payer,
    async signTypedData(data) {
      let signature = await wallet.signTypedData(data);
      if (typeof signature === 'string' && !signature.startsWith('0x')) {
        signature = `0x${signature}`;
      }
      return signature;
    },
  };

  const client = new x402Client();
  client.registerV1('base', new ExactEvmSchemeV1(signer));
  const endpoint = `${baseUrl}/x402/agents/${agentId}/${toolName}`;

  const preflight = await fetch(endpoint, {
    method: 'POST',
    headers: {
      accept: 'application/json',
      'content-type': 'application/json',
    },
    body: JSON.stringify(payload),
  });
  const preflightText = await preflight.text();
  let challengeBody = null;
  try {
    challengeBody = preflightText ? JSON.parse(preflightText) : null;
  } catch {
    challengeBody = null;
  }

  const selectedRequirement =
    challengeBody &&
    Array.isArray(challengeBody.accepts) &&
    challengeBody.accepts.length > 0
      ? challengeBody.accepts[0]
      : null;
  const normalizedRequirement = selectedRequirement
    ? {
        ...selectedRequirement,
        assetAddress: selectedRequirement.asset || null,
        asset:
          typeof selectedRequirement.asset === 'string' &&
          selectedRequirement.asset.toLowerCase() === BASE_USDC_ADDRESS
            ? 'USDC'
            : selectedRequirement.extra?.name === 'USD Coin'
              ? 'USDC'
              : selectedRequirement.asset,
      }
    : null;

  const paidFetch = wrapFetchWithPayment(fetch, client);
  const response = await paidFetch(endpoint, {
    method: 'POST',
    headers: {
      accept: 'application/json',
      'content-type': 'application/json',
    },
    body: JSON.stringify(payload),
  });

  const responseText = await response.text();
  let bodyJson = null;
  try {
    bodyJson = responseText ? JSON.parse(responseText) : null;
  } catch {
    bodyJson = null;
  }

  const paymentHeader =
    response.headers.get('PAYMENT-RESPONSE') || response.headers.get('X-PAYMENT-RESPONSE');
  let paymentResponse = null;
  if (paymentHeader) {
    try {
      paymentResponse = JSON.parse(Buffer.from(paymentHeader, 'base64').toString('utf8'));
    } catch (error) {
      fail(
        'Heurist payment response header could not be decoded.',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  process.stdout.write(
    `${JSON.stringify(
      {
        ok: response.ok,
        status: response.status,
        payer,
        endpoint,
        payment_requirement: normalizedRequirement,
        payment_response: paymentResponse,
        body_text: responseText,
        body_json: bodyJson,
      },
      null,
      2
    )}\n`
  );

  if (!response.ok) {
    process.exit(1);
  }
}

main().catch((error) => {
  fail('Heurist x402 call failed.', error instanceof Error ? error.stack ?? error.message : String(error));
});
