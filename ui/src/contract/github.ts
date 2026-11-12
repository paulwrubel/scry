import type { ExternalIdentity } from '@/generated/ExternalIdentity';

/**
 * Parse a GitHub issue or pull request URL into an ExternalIdentity.
 *
 * Mirrors `parse_github_identity` in `crates/scry-core/src/github.rs`: accepts
 * `https://github.com/{owner}/{repo}/issues|pull/{number}`, ignoring any
 * subpath, trailing slash, query string, or fragment. Owner and repo are
 * lowercased so one repository always produces one identity.
 */
export function parseGithubIdentity(input: string): ExternalIdentity | null {
  // strip the fragment before the query so a '?' inside the fragment cannot be
  // mistaken for the start of the query string
  const withoutFragment = input.split('#')[0] ?? '';
  const withoutQuery = withoutFragment.split('?')[0] ?? '';

  const prefix = 'https://github.com/';
  if (!withoutQuery.startsWith(prefix)) {
    return null;
  }

  const segments = withoutQuery.slice(prefix.length).split('/');
  const owner = segments[0] ?? '';
  const repo = segments[1] ?? '';
  const kind = segments[2] ?? '';
  const numberText = segments[3] ?? '';

  if (kind !== 'issues' && kind !== 'pull') {
    return null;
  }

  if (!isValidName(owner) || !isValidName(repo)) {
    return null;
  }

  if (!/^\d+$/.test(numberText)) {
    return null;
  }

  const number = Number(numberText);
  if (number === 0) {
    return null;
  }

  return {
    provider: 'github',
    owner: owner.toLowerCase(),
    repo: repo.toLowerCase(),
    number,
  };
}

export function githubPlaceholderTitle(identity: ExternalIdentity): string {
  return `${identity.owner}/${identity.repo}#${identity.number}`;
}

function isValidName(value: string): boolean {
  return value.length > 0 && !/\s/.test(value);
}
