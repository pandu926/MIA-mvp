import { describe, expect, it } from 'vitest';
import { buildPhaseOneCapabilityLanes, MIA_PHASE_ONE_CAPABILITY } from '@/lib/mia-capability';

describe('MIA_PHASE_ONE_CAPABILITY', () => {
  it('locks the MVP research provider to Heurist as an upstream source', () => {
    expect(MIA_PHASE_ONE_CAPABILITY.deepResearch.maturity).toBe('mvp');
    expect(MIA_PHASE_ONE_CAPABILITY.deepResearch.primaryResearchProvider).toBe('Heurist Mesh upstream research');
    expect(MIA_PHASE_ONE_CAPABILITY.deepResearch.xSignalStrategy).toBe('integrated_agents');
  });

  it('keeps the native X and Twitter API path for post-MVP expansion', () => {
    expect(MIA_PHASE_ONE_CAPABILITY.deepResearch.keepNativeXApi).toBe(true);
    expect(MIA_PHASE_ONE_CAPABILITY.deepResearch.nativeXApiStage).toBe('post_mvp');
  });

  it('keeps quick verdict free and deep research paid per report for MVP', () => {
    expect(MIA_PHASE_ONE_CAPABILITY.quickVerdict.access).toBe('free');
    expect(MIA_PHASE_ONE_CAPABILITY.deepResearch.access).toBe('paid');
    expect(MIA_PHASE_ONE_CAPABILITY.deepResearch.unlockModel).toBe('unlock_this_report');
  });
});

describe('buildPhaseOneCapabilityLanes', () => {
  it('returns two user-facing lanes with clear access boundaries', () => {
    const lanes = buildPhaseOneCapabilityLanes();

    expect(lanes).toHaveLength(2);
    expect(lanes[0]).toMatchObject({
      title: 'Free Workflow',
      accessLabel: 'Free lane',
    });
    expect(lanes[1]).toMatchObject({
      title: 'Deep Research',
      accessLabel: 'Paid lane',
      ctaLabel: 'Unlock this report',
    });
    expect(lanes[1]?.bullets.some((item) => item.includes('BANK OF AI official facilitator'))).toBe(true);
    expect(lanes[1]?.footnote).toContain('Heurist remains an upstream source');
  });
});
