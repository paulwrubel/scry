import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Task } from '@/generated/Task';

export function githubIdentity(task: Task): ExternalIdentity | null {
  return task.external_identity?.provider === 'github' ? task.external_identity : null;
}

export function githubUrl(task: Task): string | null {
  const identity = githubIdentity(task);
  if (identity === null) {
    return null;
  }
  const kind = task.external_metadata?.kind === 'pull' ? 'pull' : 'issues';
  return `https://github.com/${identity.owner}/${identity.repo}/${kind}/${identity.number}`;
}

export function githubLabel(task: Task): string | null {
  const identity = githubIdentity(task);
  return identity === null ? null : `${identity.owner}/${identity.repo}#${identity.number}`;
}

export function titleLocked(task: Task): boolean {
  return (task.external_metadata?.title ?? null) !== null;
}

export function descriptionLocked(task: Task): boolean {
  return (task.external_metadata?.description ?? null) !== null;
}

export function stateLocked(task: Task): boolean {
  return (task.external_metadata?.state ?? null) !== null;
}

export function externalError(task: Task): string | null {
  return task.external_metadata?.error ?? null;
}

export function lastSyncedAt(task: Task): string | null {
  return task.external_metadata?.updated_at ?? null;
}
