import { describe, expect, it } from 'vitest';
import type { ExternalIdentity } from '@/generated/ExternalIdentity';
import { parseTaskInput } from './input';

const ACME_WIDGETS: ExternalIdentity = {
  provider: 'github',
  owner: 'acme',
  repo: 'widgets',
  number: 123,
};

describe('parseTaskInput', () => {
  it('treats plain text as a trimmed title', () => {
    expect(parseTaskInput('  New task  ')).toEqual({ title: 'New task', externalIdentity: null });
  });

  it('parses a GitHub issue URL into a placeholder title and identity', () => {
    expect(parseTaskInput('https://github.com/acme/widgets/issues/123')).toEqual({
      title: 'acme/widgets#123',
      externalIdentity: ACME_WIDGETS,
    });
  });

  it('parses a GitHub pull URL with a subpath', () => {
    expect(parseTaskInput('https://github.com/acme/widgets/pull/123/files')).toEqual({
      title: 'acme/widgets#123',
      externalIdentity: ACME_WIDGETS,
    });
  });

  it('rejects a non-GitHub URL', () => {
    expect(parseTaskInput('https://gitlab.com/acme/widgets/issues/123')).toBeNull();
    expect(parseTaskInput('http://example.com/task')).toBeNull();
  });

  it('rejects empty and whitespace-only input', () => {
    expect(parseTaskInput('')).toBeNull();
    expect(parseTaskInput('   ')).toBeNull();
  });

  it('parses a URL surrounded by whitespace', () => {
    expect(parseTaskInput('  https://github.com/acme/widgets/issues/123  ')).toEqual({
      title: 'acme/widgets#123',
      externalIdentity: ACME_WIDGETS,
    });
  });
});
