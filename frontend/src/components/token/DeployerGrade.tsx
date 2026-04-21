import type { TrustGrade } from '@/lib/types';

interface DeployerGradeProps {
  grade: TrustGrade;
  label?: string;
  size?: 'sm' | 'md';
}

const GRADE_STYLES: Record<TrustGrade, string> = {
  A: 'bg-emerald-500/20 text-emerald-300 border-emerald-500/40',
  B: 'bg-blue-500/20 text-blue-300 border-blue-500/40',
  C: 'bg-yellow-500/20 text-yellow-300 border-yellow-500/40',
  D: 'bg-orange-500/20 text-orange-300 border-orange-500/40',
  F: 'bg-red-500/20 text-red-300 border-red-500/40',
};

export function DeployerGrade({ grade, label, size = 'md' }: DeployerGradeProps) {
  const sizeClass = size === 'sm'
    ? 'h-6 w-6 text-xs'
    : 'h-8 w-8 text-sm';

  return (
    <span className="inline-flex items-center gap-1.5">
      <span
        className={`inline-flex items-center justify-center rounded-full border font-bold ${GRADE_STYLES[grade]} ${sizeClass}`}
      >
        {grade}
      </span>
      {label && (
        <span className="text-xs text-zinc-400">{label}</span>
      )}
    </span>
  );
}
