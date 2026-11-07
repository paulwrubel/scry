import type { Color } from '@/generated/Color';

/** Hex values for the persisted section colors; shared by the column header and the picker. */
export const SECTION_COLOR_HEX: Record<Color, string> = {
  black: '#000000',
  red: '#ef4444',
  green: '#22c55e',
  yellow: '#eab308',
  blue: '#3b82f6',
  magenta: '#d946ef',
  cyan: '#06b6d4',
  gray: '#9ca3af',
  'dark-gray': '#4b5563',
  'light-red': '#fca5a5',
  'light-green': '#86efac',
  'light-yellow': '#fde047',
  'light-blue': '#93c5fd',
  'light-magenta': '#f0abfc',
  'light-cyan': '#67e8f9',
  white: '#ffffff',
};

const HUE_FAMILY_DEGREES = 20;
const NEUTRAL_SATURATION = 0.2;

type Hsl = { hue: number; saturation: number; lightness: number };

function toHsl(hex: string): Hsl {
  const red = Number.parseInt(hex.slice(1, 3), 16) / 255;
  const green = Number.parseInt(hex.slice(3, 5), 16) / 255;
  const blue = Number.parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const delta = max - min;
  const lightness = (max + min) / 2;

  if (delta === 0) {
    return { hue: 0, saturation: 0, lightness };
  }

  const saturation = delta / (1 - Math.abs(2 * lightness - 1));
  let hue: number;
  if (max === red) {
    hue = 60 * (((green - blue) / delta) % 6);
  } else if (max === green) {
    hue = 60 * ((blue - red) / delta + 2);
  } else {
    hue = 60 * ((red - green) / delta + 4);
  }

  return { hue: (hue + 360) % 360, saturation, lightness };
}

function compareByHue(left: Color, right: Color): number {
  const a = toHsl(SECTION_COLOR_HEX[left]);
  const b = toHsl(SECTION_COLOR_HEX[right]);
  const aNeutral = a.saturation < NEUTRAL_SATURATION;
  const bNeutral = b.saturation < NEUTRAL_SATURATION;

  if (aNeutral !== bNeutral) {
    return aNeutral ? 1 : -1;
  }
  if (aNeutral) {
    return b.lightness - a.lightness;
  }

  const familyA = Math.round(a.hue / HUE_FAMILY_DEGREES);
  const familyB = Math.round(b.hue / HUE_FAMILY_DEGREES);
  if (familyA !== familyB) {
    return familyA - familyB;
  }
  return a.lightness - b.lightness;
}

/** Picker order: chroma by hue, solid before its light variant, neutrals last (bright to dark). */
export const SECTION_COLORS: readonly Color[] = (Object.keys(SECTION_COLOR_HEX) as Color[]).sort(
  compareByHue,
);

export function sectionColorLabel(color: Color): string {
  const label = color.split('-').join(' ');
  return label.charAt(0).toUpperCase() + label.slice(1);
}
