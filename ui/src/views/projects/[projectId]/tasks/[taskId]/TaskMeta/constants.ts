import type { Priority } from '@/generated/Priority';

export const PRIORITY_LABELS: Record<Priority, string> = {
  critical: 'Critical',
  elevated: 'Elevated',
  normal: 'Normal',
};

export const PRIORITIES: Priority[] = ['critical', 'elevated', 'normal'];

export const PRIORITY_DOT_CLASS: Record<Exclude<Priority, 'normal'>, string> = {
  critical: 'bg-red-600',
  elevated: 'bg-yellow-300',
};

export const SELECT_CLASS =
  'border-line bg-panel text-zinc-200 cursor-pointer appearance-none rounded-full border py-1 pr-7 pl-7 text-xs outline-none disabled:cursor-not-allowed disabled:opacity-60';

export const CHEVRON_CLASS =
  'text-muted pointer-events-none absolute top-1/2 right-2 h-3 w-3 -translate-y-1/2';
