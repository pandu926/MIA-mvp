import 'dotenv/config'
import {
  X402Client,
  X402FetchClient,
  ExactPermitEvmClientMechanism,
  ExactEvmClientMechanism,
  EvmClientSigner,
  SufficientBalancePolicy,
} from '@bankofai/x402'

function parseArgs(argv) {
  const options = {
    url: process.env.SERVER_URL ?? '',
    method: process.env.X402_METHOD ?? 'GET',
    body: process.env.X402_BODY ?? '',
  }

  for (let index = 0; index < argv.length; index += 1) {
    const current = argv[index]
    const next = argv[index + 1]

    if ((current === '--url' || current === '-u') && next) {
      options.url = next
      index += 1
      continue
    }

    if ((current === '--method' || current === '-X') && next) {
      options.method = next
      index += 1
      continue
    }

    if (current === '--body' && next) {
      options.body = next
      index += 1
    }
  }

  if (!options.url) {
    throw new Error('Missing --url or SERVER_URL')
  }

  return {
    ...options,
    method: options.method.toUpperCase(),
  }
}

async function main() {
  const { url, method, body } = parseArgs(process.argv.slice(2))

  const signer = await EvmClientSigner.create()
  console.log(`Buyer wallet: ${signer.getAddress()}`)

  const x402 = new X402Client()
  x402.register('eip155:*', new ExactPermitEvmClientMechanism(signer))
  x402.register('eip155:*', new ExactEvmClientMechanism(signer))
  x402.registerPolicy(SufficientBalancePolicy)

  const client = new X402FetchClient(x402)

  const init = {
    method,
    headers: {},
  }

  if (body) {
    init.body = body
    init.headers['content-type'] = 'application/json'
  }

  const response = await client.request(url, init)

  console.log(`Status: ${response.status}`)

  const paymentResponse = response.headers.get('payment-response')
  if (paymentResponse) {
    const decoded = Buffer.from(paymentResponse, 'base64').toString('utf8')
    console.log(`Payment response: ${decoded}`)
  } else {
    console.log('Payment response: none')
  }

  const contentType = response.headers.get('content-type') ?? ''
  if (contentType.includes('application/json')) {
    console.log('Response:', JSON.stringify(await response.json(), null, 2))
    return
  }

  console.log('Response:', await response.text())
}

main().catch((error) => {
  console.error('Human x402 client failed:', error)
  process.exit(1)
})
