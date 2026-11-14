import type { Priority } from '@/generated/Priority';

/** Display order, most urgent first. */
export const PRIORITIES: Priority[] = ['critical', 'elevated', 'normal'];

export const PRIORITY_LABELS: Record<Priority, string> = {
  critical: 'Critical',
  elevated: 'Elevated',
  normal: 'Normal',
};

export const PRIORITY_DOT_CLASS: Record<Exclude<Priority, 'normal'>, string> = {
  critical: 'bg-red-600',
  elevated: 'bg-yellow-300',
};
