import {
  format as formatDateFns,
  formatDistanceToNowStrict as formatDistanceToNowStrictDateFns
} from "date-fns"

/**
 * date-fns v4's `format`/`formatDistanceToNowStrict` throw a RangeError on an
 * invalid Date instead of returning a string. Platform-sourced dates
 * (CurseForge/Modrinth fields) and other nullable timestamps aren't
 * guaranteed to parse, so every call site in the app goes through these
 * wrappers instead of calling date-fns directly, falling back to a
 * placeholder instead of crashing the render tree.
 */
function toDate(value: Date | number | string): Date {
  return value instanceof Date ? value : new Date(value)
}

function isValidDate(value: Date): boolean {
  return !Number.isNaN(value.getTime())
}

/**
 * Like date-fns `format`, but returns `fallback` instead of throwing when
 * `value` doesn't parse into a valid Date.
 */
export function safeFormat(
  value: Date | number | string,
  formatStr: string,
  fallback = "—"
): string {
  const date = toDate(value)
  return isValidDate(date) ? formatDateFns(date, formatStr) : fallback
}

/**
 * Like date-fns `formatDistanceToNowStrict`, but returns `fallback` instead
 * of throwing when `value` doesn't parse into a valid Date.
 */
export function safeFormatDistanceToNowStrict(
  value: Date | number | string,
  options?: Parameters<typeof formatDistanceToNowStrictDateFns>[1],
  fallback = "—"
): string {
  const date = toDate(value)
  return isValidDate(date)
    ? formatDistanceToNowStrictDateFns(date, options)
    : fallback
}
