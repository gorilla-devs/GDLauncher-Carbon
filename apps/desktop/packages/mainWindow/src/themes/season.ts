export const LOCALE_TAGS: Record<string, string> = {
  english: "en",
  italian: "it",
  french: "fr",
  spanish: "es",
  japanese: "ja",
  russian: "ru",
  simplified_chinese: "zh-CN"
}

function format(
  year: number,
  month: number,
  day: number,
  language: string
): string {
  const tag = LOCALE_TAGS[language] ?? "en"
  const noon = new Date(Date.UTC(year, month - 1, day, 12))
  if (
    Number.isNaN(noon.getTime()) ||
    noon.getUTCMonth() !== month - 1 ||
    noon.getUTCDate() !== day
  )
    return ""
  return new Intl.DateTimeFormat(tag, {
    month: "long",
    day: "numeric",
    timeZone: "UTC"
  }).format(noon)
}

/** "12-01" to "December 1". Year 2001 is not a leap year, so any day works. */
export function formatMonthDay(monthDay: string, language: string): string {
  const [m, d] = monthDay.split("-").map(Number)
  return format(2001, m, d, language)
}

/** "2027-01-31" to "January 31". */
export function formatIsoDay(iso: string, language: string): string {
  const [y, m, d] = iso.split("-").map(Number)
  return format(y, m, d, language)
}

/**
 * The seasonal dialog opens only when nothing else is asking for attention.
 * `betaPromptPending` is true until app.tsx has opened the beta prompt (or
 * found it isn't due); once open it is a modal, so `modalOpen` covers it.
 */
export function seasonDialogGate(s: {
  route: string
  firstLaunch: boolean | undefined
  changelog: boolean | undefined
  betaPromptPending: boolean | undefined
  modalOpen: boolean
}): boolean {
  return (
    s.route !== "/" &&
    s.firstLaunch === false &&
    s.changelog === false &&
    s.betaPromptPending === false &&
    !s.modalOpen
  )
}

export function shouldCheckOnFocus(
  lastCheckMs: number,
  nowMs: number
): boolean {
  return nowMs - lastCheckMs >= 60_000
}

/** Splits text that t() returned with name: "{{name}}". */
export function splitOnName(text: string): [string, string] {
  const i = text.indexOf("{{name}}")
  if (i < 0) return [text, ""]
  return [text.slice(0, i), text.slice(i + "{{name}}".length)]
}

/**
 * A tile is locked when the core lists its theme as unavailable. While the
 * status is still loading, only the seasonal built-ins are held back. The
 * renderer never compares dates itself.
 */
export function isThemeLocked<T extends string>(
  id: T,
  status: { themes: { id: string; available: boolean }[] } | undefined,
  isSeasonalBuiltin: (id: T) => boolean
): boolean {
  if (!status) return isSeasonalBuiltin(id)
  const entry = status.themes.find((t) => t.id === id)
  return entry !== undefined && !entry.available
}

/**
 * The dates a locked tile's tooltip shows. Once the core has answered, its
 * window, only while the theme is unavailable. While the status loads, or if
 * the query failed, a seasonal built-in is locked (see `isThemeLocked`) and
 * shows its manifest's window.
 */
export function lockedDates<T extends string>(
  id: T,
  status:
    | {
        themes: {
          id: string
          available: boolean
          from: string
          until: string
        }[]
      }
    | undefined,
  manifestWindow: (id: T) => { from: string; until: string } | null
): { from: string; until: string } | undefined {
  if (!status) return manifestWindow(id) ?? undefined
  const entry = status.themes.find((t) => t.id === id)
  return entry && !entry.available
    ? { from: entry.from, until: entry.until }
    : undefined
}

/**
 * A startup query's answer as the seasonal dialog's gate reads it: still
 * loading (undefined) blocks, but a query that failed no longer does, so one
 * failed read can't hold the dialog back for the whole session.
 */
export function gateAnswer(q: {
  data: boolean | undefined
  isError: boolean
}): boolean | undefined {
  if (q.data === undefined && q.isError) return false
  return q.data
}

/**
 * Whether the beta prompt still stands in the way of the seasonal dialog.
 * `due` is the query result (undefined while loading); the prompt only
 * blocks until app.tsx has opened it, after which the modal stack covers it.
 */
export function betaPromptPending(
  due: boolean | undefined,
  opened: boolean
): boolean | undefined {
  if (due === undefined) return undefined
  return due && !opened
}
