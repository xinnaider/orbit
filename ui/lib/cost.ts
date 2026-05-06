// src/lib/cost.ts

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return String(n);
}

export function formatTokensLong(n: number): string {
  return n.toLocaleString();
}

const INPUT_COST_PER_M = 3;
const OUTPUT_COST_PER_M = 15;

export function estimateCost(input: number, output: number): number {
  return (input / 1_000_000) * INPUT_COST_PER_M + (output / 1_000_000) * OUTPUT_COST_PER_M;
}

export function formatCost(cost: number): string {
  if (cost < 0.01) return `< $0.01`;
  return `$${cost.toFixed(3)}`;
}
