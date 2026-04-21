export type CapabilityAccess = 'free' | 'paid';
export type CapabilityUnlockModel = 'unlock_this_report' | 'day_pass';
export type CapabilityStage = 'mvp' | 'post_mvp';
export type XSignalStrategy = 'integrated_agents' | 'native_x_api';

export interface PhaseOneLane {
  title: string;
  accessLabel: string;
  summary: string;
  ctaLabel: string | null;
  bullets: string[];
  footnote: string;
}

export const MIA_PHASE_ONE_CAPABILITY = {
  quickVerdict: {
    access: 'free' as CapabilityAccess,
    promise: 'Return a complete free workflow with verdict, evidence, action route, and exit discipline before asking for payment.',
    surfaces: ['Verdict', 'Confidence', 'Action route', 'Exit plan', 'Alert preset', 'Evidence and proof'],
  },
  deepResearch: {
    access: 'paid' as CapabilityAccess,
    maturity: 'mvp' as CapabilityStage,
    unlockModel: 'unlock_this_report' as CapabilityUnlockModel,
    primaryResearchProvider: 'Heurist Mesh upstream research',
    xSignalStrategy: 'integrated_agents' as XSignalStrategy,
    keepNativeXApi: true,
    nativeXApiStage: 'post_mvp' as CapabilityStage,
    promise: 'Unlock a conviction-grade dossier only when the setup deserves deeper investigation.',
    premiumSurfaces: [
      'Integrated X and web research',
      'DexScreener market context',
      'BscScan contract enrichment',
      'Expanded holder structure',
      'Linked deployer and wallet cluster tracing',
    ],
  },
} as const;

export function buildPhaseOneCapabilityLanes(): PhaseOneLane[] {
  return [
    {
      title: 'Free Workflow',
      accessLabel: 'Free lane',
      summary: MIA_PHASE_ONE_CAPABILITY.quickVerdict.promise,
      ctaLabel: null,
      bullets: [
        'Ticker or contract lookup in one screen',
        'Clear verdict with conviction and risk',
        'Action route, exit discipline, and evidence attached',
      ],
      footnote: 'This remains free. MIA must already be useful before asking the user to unlock anything deeper.',
    },
    {
      title: 'Deep Research',
      accessLabel: 'Paid lane',
      summary: MIA_PHASE_ONE_CAPABILITY.deepResearch.promise,
      ctaLabel: 'Unlock this report',
      bullets: [
        'User pays MIA through x402 on BSC via BANK OF AI official facilitator',
        `MVP upstream research provider: ${MIA_PHASE_ONE_CAPABILITY.deepResearch.primaryResearchProvider}`,
        'Use integrated agents instead of a direct legacy X/Twitter read path for the MVP dossier',
        'Expose linked deployer, holder, and relaunch-pattern evidence with confidence labels',
      ],
      footnote: 'Native X/Twitter API stays in the stack for post-MVP expansion, outbound posting, and deeper timeline features. Heurist remains an upstream source, not the user-facing payment surface.',
    },
  ];
}
