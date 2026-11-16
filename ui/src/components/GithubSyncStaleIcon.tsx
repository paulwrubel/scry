import type { DateTime } from 'luxon';
import { HiClock } from 'react-icons/hi2';
import type { Task } from '@/generated/Task';
import { githubSyncStale, lastSyncedAt } from '@/utils/github';

export type GithubSyncStaleIconProps = {
  task: Task;
  now?: DateTime;
};

export function GithubSyncStaleIcon(props: GithubSyncStaleIconProps) {
  const { task, now } = props;

  if (!githubSyncStale(task, now)) {
    return null;
  }

  const label =
    lastSyncedAt(task) === null ? 'GitHub not synced yet' : 'GitHub not synced in over 24 hours';

  return (
    <span
      role="img"
      aria-label={label}
      title={label}
      className="inline-flex shrink-0 items-center text-amber-400"
    >
      <HiClock aria-hidden="true" className="h-3.5 w-3.5" />
    </span>
  );
}
