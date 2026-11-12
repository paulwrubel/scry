import { githubPlaceholderTitle, parseGithubIdentity } from '@/contract/github';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';

export type TaskInput = {
  title: string;
  externalIdentity: ExternalIdentity | null;
};

export function parseTaskInput(input: string): TaskInput | null {
  const trimmed = input.trim();
  if (trimmed === '') {
    return null;
  }

  const identity = parseGithubIdentity(trimmed);
  if (identity !== null) {
    return { title: githubPlaceholderTitle(identity), externalIdentity: identity };
  }

  if (/^https?:\/\//i.test(trimmed)) {
    return null;
  }

  return { title: trimmed, externalIdentity: null };
}
