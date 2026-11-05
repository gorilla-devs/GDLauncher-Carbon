import { afterEach, expect, it, vi } from "vitest"
import { createInstance } from "i18next"
import {
  createI18nOptions,
  loadLanguageFiles,
  supportedLanguages,
  type TypedTFunction
} from "@gd/i18n"
import { formatRelativeTime } from "./relativeTime"

const ids = Object.keys(supportedLanguages)
afterEach(() => vi.useRealTimers())
it.each(ids)(
  "GPT0137 selected %s relative/time/task plural families stay readable independently of host ICU",
  async (id) => {
    const english = await loadLanguageFiles("english")
    const resources = await loadLanguageFiles(id)
    const i18n = createInstance()
    await i18n.init(createI18nOptions(id, english, resources))
    vi.useFakeTimers()
    vi.setSystemTime(new Date("2026-10-06T12:00:00Z"))
    for (const count of [1, 2, 5, 21]) {
      for (const [unit, ms] of [
        ["minutes", 60000],
        ["hours", 3600000],
        ["days", 86400000]
      ] as const) {
        const output = formatRelativeTime(
          i18n.t as unknown as TypedTFunction,
          new Date(Date.now() - count * ms).toISOString()
        )
        expect(output).not.toContain("_trn_")
        expect(output).toContain(String(count))
        if (id === "english" && unit === "minutes" && count === 2)
          expect(output).toBe("2 minutes ago")
        if (id === "simplified_chinese" && unit === "minutes" && count === 2)
          expect(output).toBe("2 分钟前")
        if (id === "russian" && unit === "minutes" && count === 2)
          expect(output).toBe("2 минуты назад")
        if (id === "russian" && unit === "minutes" && count === 5)
          expect(output).toBe("5 минут назад")
      }
    }
    for (const count of [0, 1, 2, 5, 21, 1.5, 1000000]) {
      for (const family of ["minutes", "hours", "days", "weeks", "months"]) {
        const output = i18n.t(`instances:_trn_${family}_ago`, { count })
        expect(output).not.toContain("_trn_")
      }
      for (const family of ["active_tasks", "active_count"]) {
        const output = i18n.t(`tasks:_trn_task_status_${family}`, { count })
        expect(output).not.toContain("_trn_")
        expect(output).toContain(String(count))
      }
    }
    expect(formatRelativeTime(i18n.t as unknown as TypedTFunction, null)).toBe(
      resources.instances && (resources.instances as any)._trn_never_played
    )
    expect(
      formatRelativeTime(
        i18n.t as unknown as TypedTFunction,
        new Date(Date.now() - 1000).toISOString()
      )
    ).toBe(i18n.t("instances:_trn_just_now"))
  }
)

it.each(ids)(
  "%s says how many servers were restored for every count",
  async (id) => {
    const english = await loadLanguageFiles("english")
    const resources = await loadLanguageFiles(id)
    const i18n = createInstance()
    await i18n.init(createI18nOptions(id, english, resources))
    for (const count of [1, 2, 3, 5, 11, 21, 22, 25]) {
      const title = i18n.t("notifications:_trn_servers_restored", { count })
      expect(title).not.toContain("_trn_")
      expect(title).toContain(String(count))
      const hint = i18n.t("notifications:_trn_servers_restored_hint", {
        count
      })
      expect(hint).not.toContain("_trn_")
      expect(hint.length).toBeGreaterThan(20)
    }
    if (id === "english") {
      expect(i18n.t("notifications:_trn_servers_restored", { count: 1 })).toBe(
        "Restored 1 server"
      )
      expect(i18n.t("notifications:_trn_servers_restored", { count: 3 })).toBe(
        "Restored 3 servers"
      )
    }
  }
)

it.each(["toString", "__proto__", "unrecognized"])(
  "GPT0137 startup invalid stored %s falls back safely to English options",
  async (id) => {
    const english = await loadLanguageFiles("english")
    const options = createI18nOptions(id, english)
    const i18n = createInstance()
    await i18n.init(options)
    expect(i18n.language).toBe("en")
    expect(i18n.t("instances:_trn_minutes_ago", { count: 2 })).toBe(
      "2 minutes ago"
    )
  }
)
