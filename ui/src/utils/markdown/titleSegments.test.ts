import { describe, expect, it } from 'vitest';
import { parseTitleSegments } from './titleSegments';

describe('parseTitleSegments', () => {
  it('returns no segments for an empty title', () => {
    expect(parseTitleSegments('')).toEqual([]);
  });

  it('returns a single plain segment without backticks', () => {
    expect(parseTitleSegments('Fix the parser')).toEqual([{ text: 'Fix the parser', code: false }]);
  });

  it('splits a code span out of surrounding text', () => {
    expect(parseTitleSegments('Fix `parse()` crash')).toEqual([
      { text: 'Fix ', code: false },
      { text: 'parse()', code: true },
      { text: ' crash', code: false },
    ]);
  });

  it('handles a title that is only a code span', () => {
    expect(parseTitleSegments('`npm run build`')).toEqual([{ text: 'npm run build', code: true }]);
  });

  it('handles multiple code spans', () => {
    expect(parseTitleSegments('`a` and `b`')).toEqual([
      { text: 'a', code: true },
      { text: ' and ', code: false },
      { text: 'b', code: true },
    ]);
  });

  it('handles adjacent code spans', () => {
    expect(parseTitleSegments('`a``b`')).toEqual([
      { text: 'a', code: true },
      { text: 'b', code: true },
    ]);
  });

  it('keeps unbalanced backticks literal', () => {
    expect(parseTitleSegments('Fix `parse()')).toEqual([{ text: 'Fix `parse()', code: false }]);
  });

  it('keeps empty backticks literal', () => {
    expect(parseTitleSegments('a `` b')).toEqual([{ text: 'a `` b', code: false }]);
  });

  it('does not interpret other markdown', () => {
    expect(parseTitleSegments('**bold** and _italic_')).toEqual([
      { text: '**bold** and _italic_', code: false },
    ]);
  });
});
