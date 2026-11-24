// The app names languages ("simplified_chinese"), which `Intl` can't read.
const INTL_LOCALES: Record<string, string> = {
  english: "en",
  french: "fr",
  italian: "it",
  japanese: "ja",
  russian: "ru",
  simplified_chinese: "zh-CN",
  spanish: "es"
}

/** The `Intl` locale for an app language, or `undefined` for the default. */
export function intlLocale(language: string): string | undefined {
  return INTL_LOCALES[language]
}

/** Largest first. A month is 30 days and a year 365. */
const SPAN_UNITS = [
  { unit: "year", seconds: 60 * 60 * 24 * 365 },
  { unit: "month", seconds: 60 * 60 * 24 * 30 },
  { unit: "week", seconds: 60 * 60 * 24 * 7 },
  { unit: "day", seconds: 60 * 60 * 24 },
  { unit: "hour", seconds: 60 * 60 },
  { unit: "minute", seconds: 60 },
  { unit: "second", seconds: 1 }
] as const

/** "1 hour, 5 minutes, and 3 seconds", in the app language `language`. */
export function formatSeconds(seconds: number, language: string): string {
  const locale = intlLocale(language)
  const amount = (value: number, unit: string) =>
    new Intl.NumberFormat(locale, {
      style: "unit",
      unit,
      unitDisplay: "long"
    }).format(value)

  let remaining = Math.max(0, Math.floor(seconds))
  const parts: string[] = []
  for (const { unit, seconds: length } of SPAN_UNITS) {
    const value = Math.floor(remaining / length)
    if (value > 0) {
      remaining %= length
      parts.push(amount(value, unit))
    }
  }

  if (parts.length === 0) return amount(0, "second")
  return new Intl.ListFormat(locale, { type: "conjunction" }).format(parts)
}

/** "November 19, 2026", in the app language `language`, or `fallback` for
 *  a date that doesn't parse. */
export function formatLongDate(
  value: Date | number | string,
  language: string,
  fallback = "—"
): string {
  const date = value instanceof Date ? value : new Date(value)
  if (Number.isNaN(date.getTime())) return fallback

  return new Intl.DateTimeFormat(intlLocale(language), {
    dateStyle: "long"
  }).format(date)
}
