import type { AskMiaTraceStepResponse } from './types';

export const ASK_MIA_PRESET_QUESTIONS = [
  'Why is this risky?',
  'Is this organic or manufactured?',
  "Compare this with the deployer's previous launches.",
  'What should I watch in the next hour?',
  'What is the strongest reason to stay out?',
];

export const ASK_MIA_LOADING_STEPS: AskMiaTraceStepResponse[] = [
  {
    tool: 'get_token_overview',
    title: 'Token overview',
    detail: 'Resolve launch identity and baseline activity.',
  },
  {
    tool: 'get_market_structure',
    title: 'Market structure',
    detail: 'Read buy and sell pressure from the current flow.',
  },
  {
    tool: 'get_wallet_structure',
    title: 'Wallet structure',
    detail: 'Inspect holder spread and concentration.',
  },
  {
    tool: 'get_deployer_memory',
    title: 'Builder memory',
    detail: 'Check the deployer history and repeat behavior.',
  },
  {
    tool: 'get_ml_context',
    title: 'ML context',
    detail: "Attach MIA's internal ranking and proof context.",
  },
];

export function shortenAddress(value: string, head = 8, tail = 4) {
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}...${value.slice(-tail)}`;
}
