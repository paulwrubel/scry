import { Popover } from 'flowbite-react';
import type { PopoverTheme } from 'flowbite-react';
import type { DeepPartial } from 'flowbite-react/types';
import { useState } from 'react';
import type { Color } from '@/generated/Color';
import { SECTION_COLORS, SECTION_COLOR_HEX, sectionColorLabel } from '@/utils/sectionColor';

const popoverTheme: DeepPartial<PopoverTheme> = {
  base: 'absolute z-30 rounded-lg border border-line bg-panel shadow-lg outline-none',
  content: 'overflow-hidden rounded-lg',
};

const TRIGGER_CLASS =
  '-ml-1 inline-flex h-5 w-5 shrink-0 cursor-pointer items-center justify-center rounded-full border-0 bg-transparent p-0 transition-colors hover:bg-white/10 focus:outline-none focus-visible:ring-2 focus-visible:ring-white/60';

const DOT_CLASS = 'h-2.5 w-2.5 rounded-full';

const HOLLOW_DOT_CLASS = `${DOT_CLASS} border border-zinc-400/70`;

export type SectionColorPickerProps = {
  color: Color | null;
  onChange: (color: Color | null) => void;
};

export function SectionColorPicker(props: SectionColorPickerProps) {
  const { color, onChange } = props;
  const [open, setOpen] = useState(false);

  function select(next: Color | null) {
    onChange(next);
    setOpen(false);
  }

  return (
    <Popover
      arrow={false}
      placement="bottom-start"
      trigger="click"
      open={open}
      onOpenChange={setOpen}
      theme={popoverTheme}
      content={
        <div className="flex flex-col gap-2 p-2">
          <div className="grid grid-cols-4 gap-1.5" role="group" aria-label="Section color">
            {SECTION_COLORS.map((candidate) => (
              <button
                key={candidate}
                type="button"
                aria-label={sectionColorLabel(candidate)}
                title={sectionColorLabel(candidate)}
                aria-pressed={color === candidate}
                onClick={() => select(candidate)}
                className={`h-5 w-5 cursor-pointer rounded-full ring-1 ring-white/20 transition-transform hover:scale-110 focus:outline-none focus-visible:ring-2 focus-visible:ring-white ${
                  color === candidate ? 'ring-2 ring-white' : ''
                }`}
                style={{ backgroundColor: SECTION_COLOR_HEX[candidate] }}
              />
            ))}
          </div>
          <button
            type="button"
            aria-pressed={color === null}
            onClick={() => select(null)}
            className="text-muted flex cursor-pointer items-center gap-2 rounded px-1 py-1 text-xs transition-colors hover:bg-white/10 hover:text-zinc-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-white/60"
          >
            <span aria-hidden="true" className={HOLLOW_DOT_CLASS} />
            No color
          </button>
        </div>
      }
    >
      <button
        type="button"
        aria-label={
          color === null ? 'Set section color' : `Section color: ${sectionColorLabel(color)}`
        }
        title="Section color"
        className={TRIGGER_CLASS}
      >
        <span
          aria-hidden="true"
          className={color === null ? HOLLOW_DOT_CLASS : `${DOT_CLASS} ring-1 ring-white/20`}
          style={color === null ? undefined : { backgroundColor: SECTION_COLOR_HEX[color] }}
        />
      </button>
    </Popover>
  );
}
