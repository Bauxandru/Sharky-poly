/** Format APR basis points to a human-readable percentage string */
export function formatApr(bps: number): string {
  return `${(bps / 100).toFixed(1)}%`;
}

/** Format lamports to SOL with 3 decimal places */
export function formatSol(lamports: number): string {
  return (lamports / 1e9).toFixed(3);
}

/** Format duration in seconds to human-readable */
export function formatDuration(seconds: number): string {
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)}h`;
  return `${Math.round(seconds / 86400)}d`;
}

/** Format Unix timestamp to relative time */
export function formatTs(ts: number): string {
  const now = Date.now() / 1000;
  const diff = ts - now;

  if (diff < 0) return "Expired";
  if (diff < 3600) return `${Math.round(diff / 60)}m`;
  if (diff < 86400) return `${Math.round(diff / 3600)}h`;
  return `${Math.round(diff / 86400)}d`;
}
