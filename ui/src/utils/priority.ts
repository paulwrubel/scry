import type { Priority } from '@/generated/Priority';

export const PRIORITIES: Priority[] = ['critical', 'high', 'medium', 'low', 'minimal'];

export const PRIORITY_LABELS: Record<Priority, string> = {
  critical: 'Critical',
  high: 'High',
  medium: 'Medium',
  low: 'Low',
  minimal: 'Minimal',
};

/** Filled circle shown on task cards and in the priority selector. */
export const PRIORITY_DOT_CLASS: Record<Priority, string> = {
  critical: 'bg-red-600',
  high: 'bg-orange-500',
  medium: 'bg-yellow-300',
  low: 'bg-purple-500',
  minimal: 'bg-white',
};

/** Highlighted band background in the board's priority view. */
export const PRIORITY_BAND_CLASS: Record<Priority, string> = {
  critical: 'ring-red-600/50 bg-red-600/10',
  high: 'ring-orange-500/50 bg-orange-500/10',
  medium: 'ring-yellow-300/50 bg-yellow-300/10',
  low: 'ring-purple-500/50 bg-purple-500/10',
  minimal: 'ring-white/40 bg-white/5',
};

/** Band caption text in the board's priority view. */
export const PRIORITY_LABEL_CLASS: Record<Priority, string> = {
  critical: 'text-red-400',
  high: 'text-orange-400',
  medium: 'text-yellow-200',
  low: 'text-purple-300',
  minimal: 'text-zinc-200',
};
