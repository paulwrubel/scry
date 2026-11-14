export type TitleSegment = {
  text: string;
  code: boolean;
};

const CODE_SPAN = /`([^`\n]+)`/g;

/**
 * GitHub renders only backtick spans in issue titles, so titles get their own
 * splitter rather than the markdown pipeline. Unbalanced or empty backticks are
 * left literal, matching what GitHub displays.
 */
export function parseTitleSegments(title: string): TitleSegment[] {
  const segments: TitleSegment[] = [];
  let consumed = 0;

  for (const match of title.matchAll(CODE_SPAN)) {
    const start = match.index ?? consumed;

    if (start > consumed) {
      segments.push({ text: title.slice(consumed, start), code: false });
    }

    segments.push({ text: match[1], code: true });
    consumed = start + match[0].length;
  }

  if (consumed < title.length) {
    segments.push({ text: title.slice(consumed), code: false });
  }

  return segments;
}
