"use client";

export function RewardBadge({ score, rank }: { score: number; rank?: number }) {
  return (
    <div className="inline-flex items-center gap-1.5 bg-yellow-950 border border-yellow-800 rounded-full px-3 py-1 text-xs text-yellow-400">
      <span>★</span>
      <span>Score: {score.toLocaleString()}</span>
      {rank !== undefined && <span className="text-yellow-600">#{rank}</span>}
    </div>
  );
}
