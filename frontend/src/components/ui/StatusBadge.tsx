interface StatusBadgeProps {
  status: 'ok' | 'connected' | 'running' | 'error' | 'degraded' | 'idle' | string;
  label: string;
}

function getStatusColor(status: string): string {
  const normalized = status.toLowerCase();

  if (['ok', 'connected', 'running'].includes(normalized)) {
    return 'bg-green-500/15 text-green-400 border border-green-500/30';
  }

  if (['idle', 'degraded'].includes(normalized)) {
    return 'bg-yellow-500/15 text-yellow-400 border border-yellow-500/30';
  }

  if (['error', 'down', 'failed'].includes(normalized)) {
    return 'bg-red-500/15 text-red-400 border border-red-500/30';
  }

  return 'bg-gray-500/15 text-gray-400 border border-gray-500/30';
}

function getDotColor(status: string): string {
  const normalized = status.toLowerCase();

  if (['ok', 'connected', 'running'].includes(normalized)) {
    return 'bg-green-400';
  }

  if (['idle', 'degraded'].includes(normalized)) {
    return 'bg-yellow-400';
  }

  if (['error', 'down', 'failed'].includes(normalized)) {
    return 'bg-red-400';
  }

  return 'bg-gray-400';
}

export function StatusBadge({ status, label }: StatusBadgeProps) {
  const colorClass = getStatusColor(status);
  const dotColor = getDotColor(status);

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${colorClass}`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${dotColor}`} aria-hidden="true" />
      {label}
    </span>
  );
}
