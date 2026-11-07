import { describe, expect, it } from 'vitest';
import { setSectionColorOperation, updateSectionOperation } from './sectionOperations';

describe('setSectionColorOperation', () => {
  it('sets a color without touching the name', () => {
    expect(setSectionColorOperation('section-1', 'red')).toMatchObject({
      kind: 'update_section',
      id: 'section-1',
      name: null,
      color: { set: 'red' },
    });
  });

  it('clears the color', () => {
    expect(setSectionColorOperation('section-1', null)).toMatchObject({ color: 'clear' });
  });

  it('generates a fresh operation id at version 1', () => {
    const first = setSectionColorOperation('section-1', 'red');
    const second = setSectionColorOperation('section-1', 'red');

    expect(first.operation_id).not.toBe(second.operation_id);
    expect(first.operation_version).toBe(1);
  });
});

describe('updateSectionOperation', () => {
  it('leaves the color unchanged when renaming', () => {
    expect(updateSectionOperation('section-1', '  Renamed  ')).toMatchObject({
      kind: 'update_section',
      id: 'section-1',
      name: 'Renamed',
      color: null,
    });
  });
});
