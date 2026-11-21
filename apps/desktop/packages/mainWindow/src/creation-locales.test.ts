import { expect, test } from "vitest"
import i18next from "i18next"
import { createI18nOptions } from "../../../../../packages/i18n/index"
import english from "../../../../../packages/i18n/locale/english/instances.json"
import italian from "../../../../../packages/i18n/locale/italian/instances.json"
import japanese from "../../../../../packages/i18n/locale/japanese/instances.json"
import russian from "../../../../../packages/i18n/locale/russian/instances.json"
import spanish from "../../../../../packages/i18n/locale/spanish/instances.json"
import simplified_chinese from "../../../../../packages/i18n/locale/simplified_chinese/instances.json"
import french from "../../../../../packages/i18n/locale/french/instances.json"
const resources = {
  english,
  italian,
  japanese,
  russian,
  spanish,
  simplified_chinese,
  french
}
const keys = [
  "_trn_custom_create_failed",
  "_trn_custom_create_uncertain",
  "_trn_custom_prepare_failed",
  "_trn_custom_retry_preparation",
  "_trn_custom_existing_snapshot",
  "_trn_custom_view_library",
  "_trn_custom_fresh_copy",
  "_trn_custom_create_admitted",
  "_trn_custom_update_failed"
] as const
test.each(Object.keys(resources) as (keyof typeof resources)[])(
  "%s delivers all creation recovery phrases and the retained name",
  async (language) => {
    const i18n = i18next.createInstance()
    await i18n.init(
      createI18nOptions(
        language,
        { instances: english },
        { instances: resources[language] }
      )
    )
    for (const key of keys) {
      expect(resources[language][key].trim().length).toBeGreaterThan(0)
      expect(i18n.t(`instances:${key}`)).toBe(resources[language][key])
    }
    // The player knows the instance by its name; the database id is internal.
    const snapshot = i18n.t("instances:_trn_custom_existing_snapshot", {
      name: "Bob's & Alice's pack",
      interpolation: { escapeValue: false }
    })
    expect(snapshot).toContain("Bob's & Alice's pack")
    expect(snapshot).not.toContain("{{")
    expect(resources[language]._trn_custom_existing_snapshot).not.toMatch(
      /\{\{\s*id\s*\}\}/
    )
  }
)
