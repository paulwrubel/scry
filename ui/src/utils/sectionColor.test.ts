import { describe, expect, it } from 'vitest';
import { SECTION_COLORS, SECTION_COLOR_HEX, sectionColorLabel } from './sectionColor';

describe('sectionColorLabel', () => {
  it('turns enum values into display labels', () => {
    expect(sectionColorLabel('red')).toBe('Red');
    expect(sectionColorLabel('dark-gray')).toBe('Dark gray');
    expect(sectionColorLabel('light-magenta')).toBe('Light magenta');
  });
});

describe('SECTION_COLORS', () => {
  it('lists every persisted color exactly once', () => {
    expect([...SECTION_COLORS].sort()).toEqual(Object.keys(SECTION_COLOR_HEX).sort());
  });

  it('orders chroma by hue, solid before its light variant, with neutrals last', () => {
    expect(SECTION_COLORS).toEqual([
      'red',
      'light-red',
      'yellow',
      'light-yellow',
      'green',
      'light-green',
      'cyan',
      'light-cyan',
      'blue',
      'light-blue',
      'magenta',
      'light-magenta',
      'white',
      'gray',
      'dark-gray',
      'black',
    ]);
  });
});
