import { DateTime } from 'luxon';
import {
  FREQUENCY_CODES,
  WEEKDAY_CODES,
  type ByDay,
  type Frequency,
  type ParseResult,
  type RecurrencePattern,
  type Until,
} from './types';

/// Parse the supported RRULE subset, returning a readable error instead of
/// guessing when a rule names something this port does not implement.
export function parseRrule(text: string): ParseResult {
  try {
    return { pattern: parseRule(text), error: null };
  } catch (error) {
    if (error instanceof RRuleParseError) {
      return { pattern: null, error: error.message };
    }
    throw error;
  }
}

/// Render canonical RRULE text, leaving out defaults. Parsing the result yields
/// an equal pattern.
export function formatRrule(pattern: RecurrencePattern): string {
  const parts = [`FREQ=${FREQUENCY_CODES[pattern.freq]}`];

  if (pattern.interval !== 1) {
    parts.push(`INTERVAL=${pattern.interval}`);
  }
  if (pattern.byMonth.length > 0) {
    parts.push(`BYMONTH=${pattern.byMonth.join(',')}`);
  }
  if (pattern.byMonthDay.length > 0) {
    parts.push(`BYMONTHDAY=${pattern.byMonthDay.join(',')}`);
  }
  if (pattern.byDay.length > 0) {
    const days = pattern.byDay.map((day) => {
      const code = WEEKDAY_CODES[day.weekday - 1] ?? 'MO';
      return day.ordinal === null ? code : `${day.ordinal}${code}`;
    });
    parts.push(`BYDAY=${days.join(',')}`);
  }
  if (pattern.wkst !== 1) {
    parts.push(`WKST=${WEEKDAY_CODES[pattern.wkst - 1] ?? 'MO'}`);
  }
  if (pattern.count !== null) {
    parts.push(`COUNT=${pattern.count}`);
  }
  if (pattern.until !== null) {
    parts.push(`UNTIL=${formatUntil(pattern.until)}`);
  }

  return parts.join(';');
}

class RRuleParseError extends Error {}

function fail(token: string, detail: string): never {
  throw new RRuleParseError(`invalid RRULE token "${token}": ${detail}`);
}

function reject(part: string): never {
  throw new RRuleParseError(`unsupported RRULE part: ${part}`);
}

function parseRule(text: string): RecurrencePattern {
  const trimmed = text.trim();
  const body = trimmed.slice(0, 6).toUpperCase() === 'RRULE:' ? trimmed.slice(6) : trimmed;

  let freq: Frequency | null = null;
  let interval = 1;
  let byDay: ByDay[] = [];
  let byMonthDay: number[] = [];
  let byMonth: number[] = [];
  let count: number | null = null;
  let until: Until | null = null;
  let wkst = 1;

  for (const raw of body.split(';')) {
    const part = raw.trim();
    if (part.length === 0) {
      continue;
    }

    const equals = part.indexOf('=');
    if (equals < 0) {
      fail(part, 'expected KEY=VALUE');
    }
    const key = part.slice(0, equals).trim().toUpperCase();
    const value = part.slice(equals + 1);

    switch (key) {
      case 'FREQ':
        freq = parseFrequency(value.trim());
        break;
      case 'INTERVAL':
        interval = parsePositive(value, 'INTERVAL');
        break;
      case 'BYDAY':
        byDay = parseByDay(value.trim());
        break;
      case 'BYMONTHDAY':
        byMonthDay = parseMonthDays(value.trim());
        break;
      case 'BYMONTH':
        byMonth = parseMonths(value.trim());
        break;
      case 'COUNT':
        count = parsePositive(value, 'COUNT');
        break;
      case 'UNTIL':
        until = parseUntil(value);
        break;
      case 'WKST': {
        const token = value.trim();
        const weekday = parseWeekday(token);
        if (weekday === null) {
          return fail(token, 'expected a weekday like MO');
        }
        wkst = weekday;
        break;
      }
      default:
        reject(`unknown RRULE key ${key}`);
    }
  }

  if (freq === null) {
    return fail('FREQ', 'missing FREQ');
  }
  if (count !== null && until !== null) {
    reject('COUNT and UNTIL are mutually exclusive');
  }
  if (freq === 'weekly' && byDay.some((day) => day.ordinal !== null)) {
    reject('ordinal BYDAY on a WEEKLY rule');
  }

  return { freq, interval, byDay, byMonthDay, byMonth, count, until, wkst };
}

function parseFrequency(value: string): Frequency {
  switch (value.toUpperCase()) {
    case 'DAILY':
      return 'daily';
    case 'WEEKLY':
      return 'weekly';
    case 'MONTHLY':
      return 'monthly';
    case 'YEARLY':
      return 'yearly';
    default:
      return reject(`FREQ ${value.toUpperCase()}`);
  }
}

function parseWeekday(value: string): number | null {
  const index = WEEKDAY_CODES.indexOf(value.toUpperCase());
  return index < 0 ? null : index + 1;
}

function parsePositive(value: string, what: string): number {
  const token = value.trim();
  if (!/^\+?\d+$/.test(token)) {
    fail(token, `${what} must be a positive integer`);
  }
  const number = Number(token);
  if (!Number.isSafeInteger(number) || number <= 0) {
    fail(token, `${what} must be a positive integer`);
  }
  return number;
}

function parseByDay(value: string): ByDay[] {
  return value.split(',').map((raw) => {
    const token = raw.trim();
    if (token.length < 2 || [...token].some((character) => character.charCodeAt(0) > 0x7f)) {
      fail(token, 'expected a weekday like MO or -1FR');
    }
    const prefix = token.slice(0, -2);
    const weekday = parseWeekday(token.slice(-2));
    if (weekday === null) {
      return fail(token, 'expected a weekday like MO or -1FR');
    }
    if (prefix.length === 0) {
      return { ordinal: null, weekday };
    }
    if (!/^[+-]?\d+$/.test(prefix)) {
      fail(token, 'invalid weekday ordinal');
    }
    const ordinal = Number(prefix);
    if (ordinal === 0 || ordinal < -53 || ordinal > 53) {
      fail(token, 'weekday ordinal must be in -53..=53 and not 0');
    }
    return { ordinal, weekday };
  });
}

function parseMonthDays(value: string): number[] {
  return value.split(',').map((raw) => {
    const token = raw.trim();
    if (!/^[+-]?\d+$/.test(token)) {
      fail(token, 'expected a day of month');
    }
    const day = Number(token);
    if (day === 0 || Math.abs(day) > 31) {
      fail(token, 'day of month must be 1..=31 or -31..=-1');
    }
    return day;
  });
}

function parseMonths(value: string): number[] {
  return value.split(',').map((raw) => {
    const token = raw.trim();
    if (!/^[+-]?\d+$/.test(token)) {
      fail(token, 'expected a month');
    }
    const month = Number(token);
    if (month < 1 || month > 12) {
      fail(token, 'month must be in 1..=12');
    }
    return month;
  });
}

function parseUntil(value: string): Until {
  const token = value.trim();
  const upper = token.toUpperCase();

  if (upper.endsWith('Z')) {
    const body = upper.slice(0, -1);
    if (body.length !== 15 || body[8] !== 'T' || !/^\d{8}T\d{6}$/.test(body)) {
      fail(token, 'expected YYYYMMDDTHHMMSSZ');
    }
    const date = parseCompactDate(body.slice(0, 8), token);
    const time = body.slice(9);
    const datetime = DateTime.fromObject(
      {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: Number(time.slice(0, 2)),
        minute: Number(time.slice(2, 4)),
        second: Number(time.slice(4, 6)),
      },
      { zone: 'UTC' },
    );
    if (!datetime.isValid) {
      fail(token, datetime.invalidReason ?? 'invalid date');
    }
    return {
      kind: 'instant',
      timestamp: datetime.toFormat("yyyy-LL-dd'T'HH:mm:ss'Z'"),
    };
  }

  const date = parseCompactDate(upper, token);
  return { kind: 'date', date: date.toISODate() ?? '' };
}

function parseCompactDate(digits: string, token: string): DateTime {
  if (!/^\d{8}$/.test(digits)) {
    fail(token, 'expected a date as YYYYMMDD');
  }
  const date = DateTime.fromObject(
    {
      year: Number(digits.slice(0, 4)),
      month: Number(digits.slice(4, 6)),
      day: Number(digits.slice(6, 8)),
    },
    { zone: 'UTC' },
  );
  if (!date.isValid) {
    fail(token, date.invalidReason ?? 'invalid date');
  }
  return date;
}

function formatUntil(until: Until): string {
  if (until.kind === 'date') {
    return until.date.split('-').join('');
  }
  return DateTime.fromISO(until.timestamp, { setZone: true })
    .toUTC()
    .toFormat("yyyyLLdd'T'HHmmss'Z'");
}
