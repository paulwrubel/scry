import { describe, expect, it } from 'vitest';
import { describeRejectionReason } from './rejections';

describe('describeRejectionReason', () => {
  it('describes every rejection reason', () => {
    expect(describeRejectionReason('tombstoned')).toContain('deleted');
    expect(describeRejectionReason('not_found')).toContain('no longer');
    expect(describeRejectionReason('conflict')).toContain('newer');
    expect(describeRejectionReason('unsupported_version')).toContain('too old');
  });
});
