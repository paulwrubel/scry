export const MIN_HANDLE_LENGTH = 4;
export const MAX_HANDLE_LENGTH = 32;

export type HandleErrorCode =
  'empty' | 'too_short' | 'too_long' | 'misplaced_dash' | 'invalid_character';

export class HandleError extends Error {
  readonly code: HandleErrorCode;

  constructor(code: HandleErrorCode, message: string) {
    super(message);
    this.name = 'HandleError';
    this.code = code;
  }
}

const CANONICAL_DASH_POSITIONS = [8, 12, 16, 20];

export function normalizeHandle(value: string): string {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    throw new HandleError('empty', 'a handle cannot be empty');
  }

  let hex = '';
  for (let index = 0; index < trimmed.length; index += 1) {
    const character = trimmed[index];
    if (character === '-') {
      const trailing = index + 1 === trimmed.length;
      if (trailing || !CANONICAL_DASH_POSITIONS.includes(hex.length)) {
        throw new HandleError(
          'misplaced_dash',
          `a dash is only allowed after 8, 12, 16, or 20 hex characters (found one at index ${index})`,
        );
      }
    } else if (isHexCharacter(character)) {
      hex += character.toLowerCase();
    } else {
      throw new HandleError(
        'invalid_character',
        `a handle may only contain hex characters and dashes, but found ${JSON.stringify(character)} at index ${index}`,
      );
    }
  }

  if (hex.length < MIN_HANDLE_LENGTH) {
    throw new HandleError(
      'too_short',
      `a handle must contain at least ${MIN_HANDLE_LENGTH} hex characters (found ${hex.length})`,
    );
  }
  if (hex.length > MAX_HANDLE_LENGTH) {
    throw new HandleError(
      'too_long',
      `a handle must contain at most ${MAX_HANDLE_LENGTH} hex characters (found ${hex.length})`,
    );
  }

  return hex;
}

export function shortestHandles(ids: readonly string[], minLength: number): Map<string, string> {
  const bound = Math.min(Math.max(Math.trunc(minLength), MIN_HANDLE_LENGTH), MAX_HANDLE_LENGTH);
  const byHex = new Map<string, string>();

  for (const id of ids) {
    const hex = uuidHex(id);
    if (!byHex.has(hex)) {
      byHex.set(hex, id);
    }
  }

  const entries = [...byHex.entries()].sort(([left], [right]) => compareCodeUnits(left, right));
  const handles = new Map<string, string>();

  for (let index = 0; index < entries.length; index += 1) {
    const [hex, id] = entries[index];
    const previous = index > 0 ? entries[index - 1][0] : null;
    const next = index + 1 < entries.length ? entries[index + 1][0] : null;
    const shared = Math.max(
      previous === null ? 0 : commonPrefixLength(previous, hex),
      next === null ? 0 : commonPrefixLength(hex, next),
    );
    const length = Math.min(Math.max(shared + 1, bound), MAX_HANDLE_LENGTH);
    handles.set(id, hex.slice(0, length));
  }

  return handles;
}

export function shortestHandle(id: string, ids: readonly string[], minLength: number): string {
  return shortestHandles(ids, minLength).get(id) ?? uuidHex(id);
}

export function resolveHandle(ids: readonly string[], raw: string): string[] {
  const prefix = normalizeHandle(raw);
  const matches = ids.filter((id) => uuidHex(id).startsWith(prefix));

  return [...new Set(matches)].sort(compareCodeUnits);
}

function isHexCharacter(character: string): boolean {
  const code = character.charCodeAt(0);
  return (code >= 48 && code <= 57) || (code >= 65 && code <= 70) || (code >= 97 && code <= 102);
}

function uuidHex(id: string): string {
  return id.replace(/-/g, '').toLowerCase();
}

function commonPrefixLength(left: string, right: string): number {
  let shared = 0;
  while (shared < left.length && shared < right.length && left[shared] === right[shared]) {
    shared += 1;
  }
  return shared;
}

function compareCodeUnits(left: string, right: string): number {
  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }
  return 0;
}
