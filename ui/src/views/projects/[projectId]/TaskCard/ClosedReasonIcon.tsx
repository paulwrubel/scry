import { HiCheckCircle, HiChevronDoubleRight, HiNoSymbol } from 'react-icons/hi2';
import type { ClosedReason } from '@/generated/ClosedReason';
import { CLOSED_REASON_LABELS } from '@/data/taskState';

const CLOSED_REASON_ICONS = {
  completed: HiCheckCircle,
  cancelled: HiNoSymbol,
  skipped: HiChevronDoubleRight,
} as const;

const CLOSED_REASON_CLASSES = {
  completed: 'text-green-400',
  cancelled: 'text-muted',
  skipped: 'text-purple-400',
} as const;

export type ClosedReasonIconProps = {
  reason: ClosedReason | null | undefined;
};

export function ClosedReasonIcon(props: ClosedReasonIconProps) {
  const { reason } = props;

  if (reason === null || reason === undefined) {
    return null;
  }

  const Icon = CLOSED_REASON_ICONS[reason];
  const label = `Closed as ${CLOSED_REASON_LABELS[reason]}`;

  return (
    <span
      role="img"
      aria-label={label}
      title={label}
      className={`${CLOSED_REASON_CLASSES[reason]} inline-flex`}
    >
      <Icon aria-hidden="true" className="h-3.5 w-3.5" />
    </span>
  );
}
