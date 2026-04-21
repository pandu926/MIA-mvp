import type {
  DeepResearchLinkedTokenResponse,
  DeepResearchReportResponse,
  DeepResearchSectionResponse,
} from './types';

export interface LinkedLaunchEvidenceView {
  confidence: string;
  summary: string;
  evidence: string[];
  repeatedWallets: string[];
  relatedTokens: DeepResearchLinkedTokenResponse[];
}

export function getLinkedLaunchSection(
  report: DeepResearchReportResponse | null | undefined
): DeepResearchSectionResponse | null {
  if (!report) return null;
  return report.sections.find((section) => section.id === 'linked-launch-cluster') ?? null;
}

export function buildLinkedLaunchEvidenceView(
  report: DeepResearchReportResponse | null | undefined
): LinkedLaunchEvidenceView | null {
  const section = getLinkedLaunchSection(report);
  if (!section) return null;

  return {
    confidence: section.confidence ?? 'low',
    summary: section.summary,
    evidence: section.evidence ?? [],
    repeatedWallets: section.repeated_wallets ?? [],
    relatedTokens: section.related_tokens ?? [],
  };
}
