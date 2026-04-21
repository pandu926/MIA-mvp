import { describe, expect, it } from 'vitest';
import { buildLinkedLaunchEvidenceView, getLinkedLaunchSection } from '@/lib/deep-research';
import type { DeepResearchReportResponse } from '@/lib/types';

const fixtureReport: DeepResearchReportResponse = {
  token_address: '0x123',
  provider_path: 'MIA launch intelligence + optional narrative enrichment',
  status: 'ready',
  executive_summary: 'Premium report ready.',
  sections: [
    {
      id: 'optional-narrative',
      title: 'Optional narrative enrichment',
      summary: 'External narrative context.',
      stage: 'optional',
      source_agent: 'heurist',
    },
    {
      id: 'linked-launch-cluster',
      title: 'Linked launch cluster',
      summary: 'MIA sees a high-confidence likely linked launch cluster.',
      stage: 'mvp',
      source_agent: 'mia_internal_linking',
      confidence: 'high',
      evidence: ['Probable early-wallet cluster with 5 wallet(s).'],
      repeated_wallets: ['0xabc', '0xdef'],
      related_tokens: [
        {
          contract_address: '0x999',
          symbol: 'TEST',
          name: 'Test Token',
          is_rug: true,
          graduated: false,
        },
      ],
    },
  ],
  citations: [],
  source_status: {},
  generated_at: '2026-04-18T00:00:00Z',
  entitlement: null,
};

describe('deep-research helpers', () => {
  it('finds the linked launch section from a premium report', () => {
    const section = getLinkedLaunchSection(fixtureReport);

    expect(section?.id).toBe('linked-launch-cluster');
    expect(section?.source_agent).toBe('mia_internal_linking');
  });

  it('builds a linked launch evidence view with defaults', () => {
    const evidence = buildLinkedLaunchEvidenceView(fixtureReport);

    expect(evidence?.confidence).toBe('high');
    expect(evidence?.repeatedWallets).toHaveLength(2);
    expect(evidence?.relatedTokens[0]?.symbol).toBe('TEST');
  });
});
