import { describe, expect, it } from 'vitest';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { githubPlaceholderTitle, parseGithubIdentity } from './github';

const ACME_WIDGETS: ExternalIdentity = {
  provider: 'github',
  owner: 'acme',
  repo: 'widgets',
  number: 123,
};

describe('parseGithubIdentity', () => {
  it('parses accepted urls', () => {
    const urls = [
      'https://github.com/acme/widgets/issues/123',
      'https://github.com/acme/widgets/pull/123',
      'https://github.com/acme/widgets/pull/123/files',
      'https://github.com/acme/widgets/issues/123/',
      'https://github.com/acme/widgets/issues/123?foo=bar',
      'https://github.com/acme/widgets/issues/123#discussion',
      'https://github.com/ACME/Widgets/issues/123',
    ];

    for (const url of urls) {
      expect(parseGithubIdentity(url), url).toEqual(ACME_WIDGETS);
    }
  });

  it('rejects invalid urls', () => {
    const urls = [
      'https://gitlab.com/acme/widgets/issues/123',
      'http://github.com/acme/widgets/issues/123',
      'https://github.com/acme/issues/123',
      'https://github.com/acme/widgets/blob/main/README.md',
      'https://github.com/acme/widgets/issues/abc',
      'https://github.com/acme/widgets/issues/0',
      'https://github.com/ac me/widgets/issues/123',
      '',
    ];

    for (const url of urls) {
      expect(parseGithubIdentity(url), url).toBeNull();
    }
  });

  it('builds the placeholder title from owner, repo, and number', () => {
    expect(githubPlaceholderTitle(ACME_WIDGETS)).toBe('acme/widgets#123');
  });
});
