import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import type { Task } from '@/generated/Task';

export function githubIdentity(task: Task): ExternalIdentity | null {
  return task.external_identity?.provider === 'github' ? task.external_identity : null;
}

export type GithubKind = 'issue' | 'pull';

export function githubKind(task: Task): GithubKind | null {
  const kind = task.external_metadata?.kind;
  return kind === 'issue' || kind === 'pull' ? kind : null;
}

export function githubUrl(task: Task): string | null {
  const identity = githubIdentity(task);
  if (identity === null) {
    return null;
  }
  const kind = githubKind(task);
  return `https://github.com/${identity.owner}/${identity.repo}/${
    kind === 'pull' ? 'pull' : 'issues'
  }/${identity.number}`;
}

export function githubLabel(task: Task): string | null {
  const identity = githubIdentity(task);
  return identity === null ? null : `${identity.owner}/${identity.repo}#${identity.number}`;
}

export function githubIconColor(task: Task): string {
  const state = task.external_metadata?.state;
  if (state === 'open') {
    return 'text-green-400';
  }
  if (state === 'merged') {
    return 'text-purple-400';
  }
  if (state === 'closed') {
    return githubKind(task) === 'pull' ? 'text-red-400' : 'text-purple-400';
  }
  return 'text-muted';
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
