import type { CSSProperties } from 'react';

// UI-only deterministic tag colors. The CLI no longer colors tags.
const TAG_BACKGROUNDS = [
  '#ffd6e0',
  '#ffc9b9',
  '#ffe0bd',
  '#fbf0a8',
  '#dcedc1',
  '#b8e6d2',
  '#aee3e8',
  '#bbdcf7',
  '#c6cbf5',
  '#d9c7f0',
  '#efc7e8',
  '#faf7f2',
  '#d6d0c8',
  '#5c5470',
] as const;

const TAG_TEXT_DARK = '#18181b';
const TAG_TEXT_LIGHT = '#fafafa';

function relativeLuminance(red: number, green: number, blue: number): number {
  const channel = (value: number) => {
    const normalized = value / 255;
    return normalized <= 0.03928 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue);
}

function contrastText(background: string): string {
  const red = Number.parseInt(background.slice(1, 3), 16);
  const green = Number.parseInt(background.slice(3, 5), 16);
  const blue = Number.parseInt(background.slice(5, 7), 16);
  const luminance = relativeLuminance(red, green, blue);
  const lightContrast = 1.05 / (luminance + 0.05);
  const darkContrast = (luminance + 0.05) / 0.05;
  return lightContrast > darkContrast ? TAG_TEXT_LIGHT : TAG_TEXT_DARK;
}

const TAG_COLORS = TAG_BACKGROUNDS.map((background) => ({
  background,
  text: contrastText(background),
}));

// FNV-1a (offset basis 0x811c9dc5, prime 0x01000193). JS has no built-in
// synchronous string hash, and tag colors only need to be stable, not
// cryptographic. This is deliberately UI-only; the CLI no longer colors tags.
function hashTag(tag: string): number {
  let hash = 0x811c9dc5;
  for (let index = 0; index < tag.length; index += 1) {
    hash ^= tag.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}

export function tagBadgeStyle(tag: string): CSSProperties {
  const { background, text } = TAG_COLORS[hashTag(tag) % TAG_COLORS.length];
  return { backgroundColor: background, color: text };
}
