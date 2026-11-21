// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest"

const toastError = vi.fn()
const query = vi.fn()
const mutation = vi.fn((..._args: unknown[]) => Promise.resolve())
const applyResolvedTheme = vi.fn((..._args: unknown[]) => Promise.resolve())
let extractErrorCodeImpl: (e: unknown) => string | undefined = () => undefined
// set by each test through `load`; read by the mocked createQuery
let createQueryImpl: (
  opts: () => { queryKey: unknown[] }
) => unknown = () => ({})

vi.mock("@gd/ui", () => ({ toast: { error: toastError } }))
vi.mock("@gd/i18n", () => ({
  i18n: { t: (k: string) => k },
  useTransContext: () => [(k: string) => k]
}))
vi.mock("./rspcClient", async () => {
  const { QueryClient } = await import("@tanstack/solid-query")
  return {
    queryClient: new QueryClient({
      defaultOptions: { queries: { retry: false } }
    }),
    rspc: {
      useContext: () => ({ client: { query, mutation } }),
      createQuery: (opts: () => { queryKey: unknown[] }) =>
        createQueryImpl(opts)
    },
    extractErrorDisplay: (e: unknown) => String(e),
    extractErrorCode: (e: unknown) => extractErrorCodeImpl(e)
  }
})
vi.mock("../themes/apply", () => ({ applyResolvedTheme }))

// a fresh module per test, so the reported-failure Set starts empty
async function load() {
  vi.resetModules()
  const mod = await import("./theme")
  const { getBuiltinTheme } = await import("../themes/builtin")
  const solid = await import("solid-js")
  const { queryClient } = await import("./rspcClient")
  // mock factories survive resetModules, so the cache must be emptied here
  queryClient.clear()
  return { mod, solid, queryClient, getBuiltinTheme }
}

const appliedKeys = () => applyResolvedTheme.mock.calls.map((c) => c[0])

const themeHost = {
  setMenuLabels: vi.fn(),
  unsubscribe: vi.fn(),
  fireReset: () => {},
  onResetTheme(cb: () => void) {
    themeHost.fireReset = cb
    return themeHost.unsubscribe
  }
}

beforeEach(() => {
  themeHost.setMenuLabels.mockClear()
  themeHost.unsubscribe.mockClear()
  ;(window as any).themeHost = themeHost
  toastError.mockClear()
  query.mockReset()
  mutation.mockClear()
  applyResolvedTheme.mockClear()
  extractErrorCodeImpl = () => undefined
  vi.spyOn(console, "warn").mockImplementation(() => {})
})

describe("theme keys", () => {
  it("parses theme keys", async () => {
    const { mod } = await load()
    expect(mod.parseThemeKey("main")).toEqual({ kind: "builtin", id: "main" })
    expect(mod.parseThemeKey("user:snow-day")).toEqual({
      kind: "user",
      id: "snow-day"
    })
    expect(mod.parseThemeKey("user:../x")).toEqual({ kind: "unknown" })
    expect(mod.parseThemeKey(undefined)).toEqual({
      kind: "builtin",
      id: "main"
    })
  })

  it("maps legacy and unknown non-user ids to main", async () => {
    const { mod } = await load()
    for (const id of ["default", "light", "poison-green", "dracula", "neon"]) {
      expect(mod.parseThemeKey(id)).toEqual({ kind: "builtin", id: "main" })
    }
  })

  it("getThemeColor reads built-in palettes", async () => {
    const { mod } = await load()
    expect(mod.getThemeColor("win95", "primary")).toBe("rgb(0 0 128)")
    expect(mod.getThemeColor("user:x", "primary")).toBe(
      mod.getThemeColor("main", "primary")
    )
  })
})

describe("reset theme", () => {
  it("resetTheme applies main and saves it", async () => {
    const { mod } = await load()
    await mod.resetTheme()
    expect(applyResolvedTheme.mock.calls[0][0]).toBe("main")
    expect(mutation).toHaveBeenCalledWith([
      "settings.setSettings",
      { theme: { Set: "main" } }
    ])
  })

  it("initThemes wires the menu event and the labels", async () => {
    const ctx = await load()
    createQueryImpl = () => ({ data: { theme: "main" }, isSuccess: true })
    const dispose = ctx.solid.createRoot((d) => {
      ctx.mod.default()
      return d
    })
    expect(themeHost.setMenuLabels).toHaveBeenCalledWith({
      view: "window:_trn_menu_view",
      resetTheme: "window:_trn_menu_reset_theme"
    })
    themeHost.fireReset()
    await Promise.resolve()
    expect(mutation).toHaveBeenCalledTimes(1)
    dispose()
    expect(themeHost.unsubscribe).toHaveBeenCalled()
  })
})

describe("applyThemeByName", () => {
  it("applies a built-in theme by name", async () => {
    const { mod, getBuiltinTheme } = await load()
    await mod.applyThemeByName("win95")
    expect(applyResolvedTheme).toHaveBeenCalledWith(
      "win95",
      getBuiltinTheme("win95"),
      getBuiltinTheme("main"),
      { preview: false }
    )
  })

  it("loads user themes from the core", async () => {
    const { mod, getBuiltinTheme } = await load()
    const T = { ...getBuiltinTheme("main"), id: "demo" }
    query.mockResolvedValue(T)
    await mod.applyThemeByName("user:demo")
    expect(query).toHaveBeenCalledWith(["themes.getTheme", { id: "demo" }])
    expect(applyResolvedTheme).toHaveBeenCalledWith(
      "user:demo",
      T,
      getBuiltinTheme("main"),
      { preview: false }
    )
  })

  it("reuses the cached user theme until it is invalidated", async () => {
    const { mod, queryClient, getBuiltinTheme } = await load()
    query.mockResolvedValue({ ...getBuiltinTheme("main"), id: "demo" })
    await mod.applyThemeByName("user:demo")
    await mod.applyThemeByName("user:demo")
    expect(query).toHaveBeenCalledTimes(1)
    await queryClient.invalidateQueries({ queryKey: ["themes.getTheme"] })
    await mod.applyThemeByName("user:demo")
    expect(query).toHaveBeenCalledTimes(2)
  })

  it("the fallback toast gives the translated reason", async () => {
    const { mod } = await load()
    extractErrorCodeImpl = () => "THEME_TOO_NEW"
    query.mockRejectedValue("boom")
    await mod.applyThemeByName("user:demo")
    expect(toastError).toHaveBeenCalledWith("settings:_trn_theme_load_failed", {
      description: "settings:_trn_theme_error_too_new"
    })
  })

  it("falls back once per failure and recovers", async () => {
    const { mod, getBuiltinTheme } = await load()
    query.mockRejectedValue("boom")
    await mod.applyThemeByName("user:demo")
    await mod.applyThemeByName("user:demo")
    expect(appliedKeys()).toEqual(["main", "main"])
    expect(toastError).toHaveBeenCalledTimes(1)

    query.mockRejectedValue("bang")
    await mod.applyThemeByName("user:demo")
    expect(toastError).toHaveBeenCalledTimes(2)

    const T = { ...getBuiltinTheme("main"), id: "demo" }
    query.mockResolvedValue(T)
    await mod.applyThemeByName("user:demo")
    expect(applyResolvedTheme).toHaveBeenLastCalledWith(
      "user:demo",
      T,
      getBuiltinTheme("main"),
      { preview: false }
    )
  })

  it("a legacy or unknown built-in id applies main without a toast", async () => {
    const { mod, getBuiltinTheme } = await load()
    await mod.applyThemeByName("dracula")
    await mod.applyThemeByName("neon")
    expect(applyResolvedTheme).toHaveBeenCalledWith(
      "main",
      getBuiltinTheme("main"),
      getBuiltinTheme("main"),
      { preview: false }
    )
    expect(appliedKeys()).toEqual(["main", "main"])
    expect(toastError).not.toHaveBeenCalled()
  })

  it("a malformed user key falls back to main with a toast", async () => {
    const { mod } = await load()
    await mod.applyThemeByName("user:../x")
    expect(appliedKeys()).toEqual(["main"])
    expect(toastError).toHaveBeenCalledTimes(1)
    expect(toastError).toHaveBeenCalledWith(
      "settings:_trn_theme_load_failed",
      undefined
    )
  })

  it("a hover preview applies with the preview flag, its fallback too", async () => {
    const { mod, getBuiltinTheme } = await load()
    const main = getBuiltinTheme("main")
    const preview = { preview: true }
    await mod.applyThemeByName("win95", preview)
    expect(applyResolvedTheme).toHaveBeenLastCalledWith(
      "win95",
      getBuiltinTheme("win95"),
      main,
      preview
    )

    const T = { ...main, id: "demo" }
    query.mockResolvedValue(T)
    await mod.applyThemeByName("user:demo", preview)
    expect(applyResolvedTheme).toHaveBeenLastCalledWith(
      "user:demo",
      T,
      main,
      preview
    )

    query.mockRejectedValue("gone")
    await mod.applyThemeByName("user:gone", preview)
    expect(applyResolvedTheme).toHaveBeenLastCalledWith(
      "main",
      main,
      main,
      preview
    )
    await mod.applyThemeByName("user:../x", preview)
    expect(applyResolvedTheme).toHaveBeenLastCalledWith(
      "main",
      main,
      main,
      preview
    )

    // the leave that restores the saved theme commits it
    await mod.applyThemeByName("win95")
    expect(applyResolvedTheme).toHaveBeenLastCalledWith(
      "win95",
      getBuiltinTheme("win95"),
      main,
      { preview: false }
    )
  })

  it("discards a late user-theme response after a newer request", async () => {
    const { mod, getBuiltinTheme } = await load()
    let resolveA!: (v: unknown) => void
    query.mockReturnValue(new Promise((r) => (resolveA = r)))
    const slow = mod.applyThemeByName("user:a")
    await mod.applyThemeByName("pixelato")
    resolveA({ ...getBuiltinTheme("main"), id: "a" })
    await slow
    expect(appliedKeys()).toEqual(["pixelato"])
    expect(toastError).not.toHaveBeenCalled()
  })

  it("discards a late user-theme failure after a newer request", async () => {
    const { mod } = await load()
    let rejectA!: (v: unknown) => void
    query.mockReturnValue(new Promise((_, r) => (rejectA = r)))
    const slow = mod.applyThemeByName("user:late")
    await mod.applyThemeByName("pixelato")
    rejectA("nope")
    await slow
    expect(appliedKeys()).toEqual(["pixelato"])
    expect(toastError).not.toHaveBeenCalled()
  })
})

describe("initThemes", () => {
  async function setup(key = "user:demo") {
    const ctx = await load()
    const { createSignal, createRoot } = ctx.solid
    const [settingsKey, setSettingsKey] = createSignal<string | undefined>(key)
    const [theme, setTheme] = createSignal<{
      data?: unknown
      isError: boolean
      error?: unknown
      isFetching: boolean
    }>({ isError: false, isFetching: true })
    createQueryImpl = (opts) =>
      opts().queryKey[0] === "settings.getSettings"
        ? {
            get data() {
              return { theme: settingsKey() }
            },
            get isSuccess() {
              return true
            }
          }
        : {
            get data() {
              return theme().data
            },
            get isError() {
              return theme().isError
            },
            get error() {
              return theme().error
            },
            get isFetching() {
              return theme().isFetching
            }
          }
    createRoot(() => ctx.mod.default())
    const main = ctx.getBuiltinTheme("main")
    return { ...ctx, main, setSettingsKey, setTheme }
  }

  it("re-applies after a failed refetch and recovers", async () => {
    const { main, setTheme } = await setup()
    const T = { ...main, id: "demo" }
    // still loading: the boot theme stays
    expect(applyResolvedTheme).not.toHaveBeenCalled()

    setTheme({ data: T, isError: false, isFetching: false })
    expect(applyResolvedTheme).toHaveBeenLastCalledWith("user:demo", T, main)

    // watcher refetch fails; the query keeps the old data
    setTheme({ data: T, isError: false, isFetching: true })
    setTheme({ data: T, isError: true, error: "typo", isFetching: false })
    expect(appliedKeys().at(-1)).toBe("main")
    expect(toastError).toHaveBeenCalledTimes(1)

    // the author fixes it
    const T2 = { ...T, version: "2" }
    setTheme({ data: T, isError: true, error: "typo", isFetching: true })
    setTheme({ data: T2, isError: false, isFetching: false })
    expect(applyResolvedTheme).toHaveBeenLastCalledWith("user:demo", T2, main)
    expect(toastError).toHaveBeenCalledTimes(1)
  })

  it("skips a refetch with identical data and applies a changed theme", async () => {
    const { main, setTheme } = await setup()
    const T = { ...main, id: "demo" }
    setTheme({ data: T, isError: false, isFetching: false })
    expect(applyResolvedTheme).toHaveBeenCalledTimes(1)

    // an invalidation refetches; solid-query hands back a new, equal object
    setTheme({ data: T, isError: false, isFetching: true })
    setTheme({ data: structuredClone(T), isError: false, isFetching: false })
    expect(applyResolvedTheme).toHaveBeenCalledTimes(1)

    const T2 = { ...T, version: "2" }
    setTheme({ data: T2, isError: false, isFetching: true })
    setTheme({ data: T2, isError: false, isFetching: false })
    expect(applyResolvedTheme).toHaveBeenCalledTimes(2)
    expect(applyResolvedTheme).toHaveBeenLastCalledWith("user:demo", T2, main)
  })

  it("a refetch that differs only in an asset URL version re-applies", async () => {
    const { main, setTheme } = await setup()
    const url = (v: number) => `/theme/asset?id=demo&path=a.css&v=${v}`
    const T = { ...main, id: "demo", css: url(1) }
    setTheme({ data: T, isError: false, isFetching: false })
    setTheme({ data: T, isError: false, isFetching: true })
    setTheme({
      data: { ...structuredClone(T), css: url(2) },
      isError: false,
      isFetching: false
    })
    expect(applyResolvedTheme).toHaveBeenCalledTimes(2)
  })

  it("a landing query never overrides a newer hover preview", async () => {
    const { main, setTheme, mod } = await setup()
    await mod.applyThemeByName("pixelato")
    setTheme({
      data: { ...main, id: "demo" },
      isError: false,
      isFetching: false
    })
    expect(appliedKeys()).toEqual(["pixelato"])
  })

  it("applies built-in and legacy settings keys", async () => {
    const { setSettingsKey } = await setup("win95")
    expect(appliedKeys()).toEqual(["win95"])
    setSettingsKey("dracula")
    expect(appliedKeys()).toEqual(["win95", "main"])
    expect(toastError).not.toHaveBeenCalled()
  })
})
