import { toast } from "@gd/ui"
import { i18n, useTransContext } from "@gd/i18n"
import type { FEResolvedTheme } from "@gd/core_module/bindings"
import { createEffect, onCleanup } from "solid-js"
import { unwrap } from "solid-js/store"
import {
  extractErrorCode,
  extractErrorDisplay,
  queryClient,
  rspc
} from "./rspcClient"
import { themeErrorKey } from "../themes/errors"
import {
  BUILTIN_THEME_IDS,
  getBuiltinTheme,
  isBuiltinThemeId,
  type BuiltinThemeId
} from "../themes/builtin"
import { applyResolvedTheme } from "../themes/apply"
import type { PaletteToken } from "../themes/palette"

export type ParsedThemeKey =
  | { kind: "builtin"; id: BuiltinThemeId }
  | { kind: "user"; id: string }
  | { kind: "unknown" }

const USER_PREFIX = "user:"
const USER_ID = /^[a-z0-9]+(-[a-z0-9]+)*$/

// Ids stored by releases up to v2.0.21; they were never migrated
const LEGACY_IDS = new Set(["default", "light", "poison-green", "dracula"])

export function parseThemeKey(key: string | undefined): ParsedThemeKey {
  if (key === undefined || key === "" || LEGACY_IDS.has(key)) {
    return { kind: "builtin", id: "main" }
  }
  if (isBuiltinThemeId(key)) return { kind: "builtin", id: key }
  if (key.startsWith(USER_PREFIX)) {
    const id = key.slice(USER_PREFIX.length)
    if (id.length <= 64 && USER_ID.test(id)) return { kind: "user", id }
    return { kind: "unknown" }
  }
  // anything else that isn't an installed-theme key is just the default
  return { kind: "builtin", id: "main" }
}

// Every request (settings change, hover preview, reset, watcher invalidation)
// takes a new generation, so a slow response can't override a newer request.
let generation = 0

const reported = new Set<string>()

// The user theme initThemes last put on the page, by key and serialised
// content. Any other apply (hover preview, Reset, a failure fallback) clears
// it, so a later refetch puts the theme back; a refetch with equal content is
// skipped, which keeps the effects from restarting for nothing. solid-query
// turns structural sharing off, so every refetch is a new object and the
// comparison has to be by content. Asset URLs carry a version (`v=`), so an
// edited file still differs.
let initApplied: { key: string; content: string } | null = null

const baseTheme = () => getBuiltinTheme("main")

// `preview` (a tile's hover, and the fallback of a failed one) leaves
// committedTheme() on the saved theme; every other apply commits.
export function reportThemeFailure(
  key: string,
  error?: unknown,
  preview = false
): void {
  initApplied = null
  void applyResolvedTheme("main", baseTheme(), baseTheme(), { preview })
  const detail = error === undefined ? undefined : extractErrorDisplay(error)
  const dedupe = `${key}\n${detail ?? ""}`
  console.warn(`Theme ${key} could not be loaded`, error)
  if (reported.has(dedupe)) return
  reported.add(dedupe)
  toast.error(
    i18n.t("settings:_trn_theme_load_failed"),
    error === undefined
      ? undefined
      : { description: i18n.t(themeErrorKey(extractErrorCode(error))) }
  )
}

function applyBuiltin(id: BuiltinThemeId, preview = false) {
  return applyResolvedTheme(id, getBuiltinTheme(id), baseTheme(), { preview })
}

// Same key as the query in initThemes, so the cached result is shared and the
// watcher's invalidation makes the next fetch go back to the core.
const themeQueryKey = (id: string) => ["themes.getTheme", { id }] as const

/**
 * Puts a theme on the page by its settings key. `preview` is for the tiles'
 * hover: it shows the theme but leaves committedTheme() (the sound players,
 * the layout) on the saved one.
 */
export async function applyThemeByName(
  key: string | undefined,
  opts: { preview?: boolean } = {}
): Promise<void> {
  const gen = ++generation
  initApplied = null
  const preview = opts.preview === true
  const parsed = parseThemeKey(key)

  if (parsed.kind === "builtin") return applyBuiltin(parsed.id, preview)
  if (parsed.kind === "unknown") {
    return reportThemeFailure(key!, undefined, preview)
  }

  let theme: FEResolvedTheme
  try {
    const client = rspc.useContext().client
    theme = await queryClient.fetchQuery({
      queryKey: themeQueryKey(parsed.id),
      queryFn: () => client.query(["themes.getTheme", { id: parsed.id }]),
      staleTime: Infinity
    })
  } catch (error) {
    if (gen === generation) reportThemeFailure(key!, error, preview)
    return
  }
  if (gen !== generation) return
  return applyResolvedTheme(key!, theme, baseTheme(), { preview })
}

// The escape hatch for a theme that hides the Settings button: applies main at
// once, then saves it (a broken theme alone never rewrites the setting).
export async function resetTheme(): Promise<void> {
  generation++
  initApplied = null
  await applyResolvedTheme("main", baseTheme(), baseTheme())
  await rspc
    .useContext()
    .client.mutation(["settings.setSettings", { theme: { Set: "main" } }])
}

export function getAvailableThemes(): BuiltinThemeId[] {
  return [...BUILTIN_THEME_IDS]
}

export function getThemeColor(id: string, token: PaletteToken): string {
  const parsed = parseThemeKey(id)
  const theme = getBuiltinTheme(parsed.kind === "builtin" ? parsed.id : "main")
  return `rgb(${theme.colors[token] ?? baseTheme().colors[token]})`
}

const initThemes = () => {
  const [t] = useTransContext()
  // the menu is native, so main needs the localized labels from the page
  createEffect(() => {
    window.themeHost.setMenuLabels({
      view: t("window:_trn_menu_view"),
      resetTheme: t("window:_trn_menu_reset_theme")
    })
  })
  const stopReset = window.themeHost.onResetTheme(() => {
    void resetTheme().catch((error) => {
      console.warn("Reset theme failed", error)
    })
  })
  onCleanup(stopReset)

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))
  const key = () => settings.data?.theme
  const userId = () => {
    const parsed = parseThemeKey(key())
    return parsed.kind === "user" ? parsed.id : null
  }

  // the core's watcher invalidates this when the theme's files change
  const userTheme = rspc.createQuery(() => ({
    queryKey: ["themes.getTheme", { id: userId() ?? "" }],
    enabled: userId() !== null
  }))

  // The effect takes a generation when the key changes or a fetch starts, and
  // only applies the outcome if nothing newer (hover, Reset) came in since.
  let lastKey: string | undefined
  let wasFetching = false
  let effectGen = -1
  let handledGen = -1
  createEffect(() => {
    const k = key()
    // wait for the settings before touching the page: the boot theme is on
    if (k === undefined && !settings.isSuccess) return
    const parsed = parseThemeKey(k)
    const fetching = userTheme.isFetching
    if (k !== lastKey || (fetching && !wasFetching)) effectGen = ++generation
    lastKey = k
    wasFetching = fetching

    if (handledGen === effectGen || generation !== effectGen) return
    if (parsed.kind === "builtin") {
      handledGen = effectGen
      initApplied = null
      void applyBuiltin(parsed.id)
    } else if (parsed.kind === "unknown") {
      handledGen = effectGen
      reportThemeFailure(k!)
    } else if (!fetching) {
      // an error comes first: a failed refetch keeps the previous data
      if (userTheme.isError) {
        handledGen = effectGen
        reportThemeFailure(`user:${parsed.id}`, userTheme.error)
      } else if (userTheme.data) {
        handledGen = effectGen
        const appliedKey = `user:${parsed.id}`
        const data = userTheme.data
        const content = JSON.stringify(unwrap(data))
        if (
          initApplied?.key === appliedKey &&
          initApplied.content === content
        ) {
          return
        }
        initApplied = { key: appliedKey, content }
        void applyResolvedTheme(appliedKey, data, baseTheme())
      }
    }
  })
}

export default initThemes
