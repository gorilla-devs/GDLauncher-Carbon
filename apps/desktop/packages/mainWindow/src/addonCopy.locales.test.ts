import { readFileSync } from "node:fs"
import { describe, expect, it } from "vitest"
import { createInstance } from "i18next"
import {
  createI18nOptions,
  languageLocaleCodes,
  loadLanguageFiles
} from "@gd/i18n"
import {
  getImportErrorTranslationKey,
  getTaskTranslationKey
} from "../../../../../packages/i18n/helpers"

// This addon, datapack, search, login, cache and updater copy once shipped
// in English only. Every key below is shown to players, so each of the seven
// shipped languages must carry its own text.
const LANGUAGES = [
  "english",
  "french",
  "italian",
  "japanese",
  "russian",
  "simplified_chinese",
  "spanish"
] as const

const KEYS: Record<string, readonly string[]> = {
  content: [
    "_trn_datapack_downloaded",
    "_trn_datapack_switch_hint",
    "_trn_mods_updater.update_failed",
    "_trn_mods_updater.success",
    "_trn_mods_updater.success_datapacks",
    "_trn_mods_updater.not_started_description",
    "_trn_addon_failures.button",
    "_trn_addon_failures.title",
    "_trn_addon_failures.refresh_failed",
    "_trn_addon_failures.empty",
    "_trn_addon_failures.details",
    "_trn_addon_failures.dismiss_failed",
    "_trn_addon_failures.dismiss",
    "_trn_addon_failures.dismiss_all",
    "_trn_addon_failures.dismiss_all_failed",
    "_trn_addon_update_failed",
    "_trn_duplicated_mods_locked_message",
    "_trn_datapack_staging.added",
    "_trn_datapack_row_subtitle",
    "_trn_datapack_toggle_hint",
    "_trn_datapack_update_hint",
    "_trn_datapack_delete_hint",
    "_trn_worlds.none",
    "_trn_worlds.load_failed",
    "_trn_worlds.unreadable",
    "_trn_worlds.datapack_count_one",
    "_trn_worlds.datapack_count_other",
    "_trn_worlds.no_datapacks",
    "_trn_worlds.datapack",
    "_trn_worlds.add_datapacks",
    "_trn_worlds.update_here",
    "_trn_worlds.update_everywhere",
    "_trn_worlds.update_failed_in",
    "_trn_worlds.instance_copy",
    "_trn_worlds.download_to_instance",
    "_trn_worlds.busy",
    "_trn_worlds.add_to_world",
    "_trn_worlds.replace_title",
    "_trn_worlds.replace_files",
    "_trn_worlds.replace",
    "_trn_worlds.added_to_world",
    "_trn_worlds.no_datapack_for_version",
    "_trn_worlds.refusal.world_gone",
    "_trn_worlds.refusal.world_changed",
    "_trn_worlds.refusal.name_taken",
    "_trn_worlds.refusal.datapack_gone",
    "_trn_worlds.refusal.datapack_changed",
    "_trn_worlds.refusal.other_world",
    "_trn_worlds.refusal.folder_not_replaced",
    "_trn_worlds.refusal.not_a_datapack_version",
    "_trn_worlds.refusal.not_a_zip",
    "_trn_worlds.refusal.invalid_datapack",
    "_trn_worlds.refusal.not_downloadable",
    "_trn_worlds.refusal.download_failed",
    "_trn_worlds.refusal.no_update",
    "_trn_worlds.refusal.instance_datapack_gone",
    "_trn_worlds.refusal.name_invalid",
    "_trn_worlds.refusal.rename_incomplete",
    "_trn_worlds.refusal.not_renamed",
    "_trn_worlds.refusal.holds_link",
    "_trn_worlds.refusal.no_space",
    "_trn_worlds.refusal.changed_while_copied",
    "_trn_worlds.refusal.being_copied"
  ],
  ui: ["_trn_worlds"],
  notifications: [
    "_trn_addon_downloading_to_server",
    "_trn_addon_installed",
    "_trn_addon_install_failed",
    "_trn_shader_install_cancelled",
    "_trn_shader_install_cancelled_mid_install",
    "_trn_shader_preflight_failed",
    "_trn_datapack_downloaded",
    "_trn_addon_outcome_unavailable",
    "_trn_addon_attempt_dismissed",
    "_trn_pack_version_failed",
    "_trn_pack_version_retry",
    "_trn_pack_details_unavailable",
    "_trn_pack_no_file",
    "_trn_pack_no_versions",
    "_trn_server_pack_unavailable",
    "_trn_server_pack_file_invalid",
    "_trn_curseforge_file_unavailable",
    "_trn_curseforge_file_invalid",
    "_trn_world_saved_as_copy_hint"
  ],
  instances: [
    "_trn_server_legacy_datapack",
    "_trn_server_legacy_datapack_move",
    "_trn_status_installing",
    "_trn_status_downloading",
    "_trn_custom_prepare_failed",
    "_trn_custom_retry_preparation",
    "_trn_custom_existing_snapshot",
    "_trn_custom_create_admitted",
    "_trn_confirm_world_deletion_title",
    "_trn_confirm_world_deletion_body",
    "_trn_confirm_world_deletion_dont_ask",
    "_trn_confirm_world_deletion_cancel",
    "_trn_confirm_world_deletion_confirm",
    "_trn_confirm_world_deletion_confirm_selection",
    "_trn_import_scan_failed",
    "_trn_import_retry",
    "_trn_import_batch_uncertain",
    "_trn_import_source_unavailable",
    "_trn_import_java_path_skipped",
    "_trn_import_commands_skipped",
    "_trn_import_game_dir_missing",
    "_trn_import_unknown_minecraft_version",
    "_trn_import_hmcl_relative_run_dir",
    "_trn_import_error_bad_config",
    "_trn_import_config_unreadable",
    "_trn_import_error_archive_malformed",
    "_trn_import_error_archive_missing_manifest",
    "_trn_import_error_archive_malformed_manifest",
    "_trn_import_error_not_minecraft_modpack",
    "_trn_server_auto_restart_abandoned",
    "_trn_server_install_failed",
    "_trn_change_version_failed",
    "_trn_server_recovered_running_notice_force",
    "_trn_server_force_stop_body",
    "_trn_server_status_installing",
    "_trn_server_status_deleting",
    "_trn_server_recovered_refusal_running_force"
  ],
  java: [
    "_trn_instance_settings.java_override_mismatch",
    "_trn_runtime_relocation_busy.previous_process",
    "_trn_runtime_relocation_busy.hook"
  ],
  logs: ["_trn_adopted_no_live_log"],
  settings: [
    "_trn_show_world_deletion_warning_title",
    "_trn_show_world_deletion_warning_text"
  ],
  tasks: [
    "_trn_instance_task_stage_datapack",
    "_trn_server_task_install_mod",
    "_trn_instance_task_check_pack_origin",
    "_trn_instance_task_check_pack_origin_versions",
    "_trn_instance_task_duplicate_world"
  ],
  search: [
    "_trn_categories_load_failed",
    "_trn_curseforge_single_version",
    "_trn_search_versions_placeholder",
    "_trn_search_modloaders_placeholder",
    "_trn_selected_version_loading",
    "_trn_selected_version_unavailable",
    "_trn_selected_version_failed",
    "_trn_linked_pack_file_failed"
  ],
  modals: [
    "_trn_cache_cleanup.failed_generic",
    "_trn_cache_cleanup.outcome_unavailable",
    "_trn_cache_cleanup.retry_failed",
    "_trn_cache_cleanup.retry"
  ],
  accounts: ["_trn_gdl_account_changed_reopen", "_trn_gdl_account_gone_close"],
  auth: [
    "_trn_profile_creation.verification_failed",
    "_trn_profile_creation.retry_verification",
    "_trn_login.no_active_account",
    "_trn_login.create_account_failed"
  ],
  app: [
    "_trn_update_install_failed_description",
    "_trn_update_prepare_failed_description",
    "_trn_update_download_unavailable_description",
    "_trn_update_changed_description",
    "_trn_update_connection_failed_description",
    "_trn_update_check_error_description",
    "_trn_update_unavailable_no_download_description",
    "_trn_update_unavailable_check_failed_description",
    "_trn_shutdown_prompt.work_cancelled",
    "_trn_shutdown_prompt.blocker_hook",
    "_trn_shutdown_prompt.hook_warning"
  ],
  general: ["_trn_close"]
}

const bundle = (language: string, namespace: string) =>
  JSON.parse(
    readFileSync(
      new URL(
        `../../../../../packages/i18n/locale/${language}/${namespace}.json`,
        import.meta.url
      ),
      "utf8"
    )
  ) as Record<string, string>

const placeholders = (text: string) =>
  [...text.matchAll(/\{\{\s*([^}\s]+)\s*\}\}/g)].map((match) => match[1])

const cases = LANGUAGES.flatMap((language) =>
  Object.entries(KEYS).flatMap(([namespace, keys]) =>
    keys.map((key) => [language, `${namespace}:${key}`] as const)
  )
)

describe("addon and account copy is translated", () => {
  it.each(cases)("%s has %s with every English placeholder", (language, id) => {
    const [namespace, key] = id.split(":")
    const english = bundle("english", namespace)[key]
    const text = bundle(language, namespace)[key]
    expect(english, `english ${id}`).toBeTypeOf("string")
    expect(text, `${language} ${id}`).toBeTypeOf("string")
    expect(text.trim()).not.toBe("")
    expect(text).not.toContain("[ UNTRANSLATED ]")
    for (const name of placeholders(english))
      expect(text, `${language} ${id} keeps {{${name}}}`).toContain(
        `{{${name}}}`
      )
  })

  it("English copy is in player language, not internal terms", () => {
    const jargon = /\bstag(e|ed|ing)\b|\badmitted\b|staging folder/i
    for (const [namespace, keys] of Object.entries(KEYS)) {
      const english = bundle("english", namespace)
      for (const key of keys)
        expect(english[key], `${namespace}:${key}`).not.toMatch(jargon)
    }
  })

  it.each(LANGUAGES)(
    "%s server addon task title leaves out the backend's English label",
    (language) => {
      // The server install task's mod_name is the fixed English text
      // "Server addon and required dependencies", never an addon name.
      expect(bundle(language, "tasks")._trn_server_task_install_mod).toMatch(
        /\{\{server_name\}\}/
      )
      expect(
        bundle(language, "tasks")._trn_server_task_install_mod
      ).not.toMatch(/\{\{\s*mod_name\s*\}\}/)
    }
  )

  it.each(LANGUAGES)(
    "%s labels removing a failure differently from closing the panel",
    (language) => {
      expect(
        bundle(language, "content")["_trn_addon_failures.dismiss"]
      ).not.toBe(bundle(language, "general")._trn_close)
    }
  )

  it.each(LANGUAGES)(
    "%s offers the in-game /stop next to Force stop for a server left running by an earlier session",
    (language) => {
      const instances = bundle(language, "instances")
      expect(instances._trn_server_force_stop_body).toContain("/stop")
      expect(instances._trn_server_recovered_running_notice_force).toContain(
        "/stop"
      )
      expect(instances._trn_server_recovered_refusal_running_force).toContain(
        "/stop"
      )
    }
  )

  it.each(LANGUAGES)(
    "%s counts the updates a closed updater did not start in its own words",
    async (language) => {
      const english = await loadLanguageFiles("english")
      const i18n = createInstance()
      await i18n.init(
        createI18nOptions(language, english, await loadLanguageFiles(language))
      )
      const englishForms = Object.entries(bundle("english", "content"))
        .filter(([key]) =>
          /^_trn_mods_updater\.not_started_(one|other)$/.test(key)
        )
        .map(([, text]) => text)
      for (const count of [1, 2, 5, 21]) {
        const text = i18n.t("content:_trn_mods_updater.not_started", { count })
        expect(text).not.toContain("_trn_")
        expect(text).toContain(String(count))
        if (language !== "english")
          for (const form of englishForms)
            expect(text).not.toBe(form.replace("{{count}}", String(count)))
      }
      if (language === "english") {
        expect(
          i18n.t("content:_trn_mods_updater.not_started", { count: 1 })
        ).toBe("1 addon was not updated")
        expect(
          i18n.t("content:_trn_mods_updater.not_started", { count: 2 })
        ).toBe("2 addons were not updated")
      }
    }
  )

  // The forms i18next picks among for a count in each language.
  const COUNT_FORMS: Record<(typeof LANGUAGES)[number], readonly string[]> = {
    english: ["one", "other"],
    french: ["one", "many", "other"],
    italian: ["one", "many", "other"],
    japanese: ["one", "other"],
    russian: ["one", "few", "many", "other"],
    simplified_chinese: ["one", "other"],
    spanish: ["one", "many", "other"]
  }
  it.each(LANGUAGES)(
    "%s has every plural form of a world's data pack count",
    (language) => {
      const content = bundle(language, "content")
      for (const form of COUNT_FORMS[language]) {
        const text = content[`_trn_worlds.datapack_count_${form}`]
        expect(text, `${language} ${form}`).toBeTypeOf("string")
        expect(text).toContain("{{count}}")
      }
      // Every form the language's rules can choose is present.
      const rules = new Intl.PluralRules(languageLocaleCodes[language])
      for (const category of rules.resolvedOptions().pluralCategories)
        expect(
          content[`_trn_worlds.datapack_count_${category}`],
          `${language} ${category}`
        ).toBeTypeOf("string")
    }
  )

  it.each(LANGUAGES)(
    "%s says a world saved as a copy is listed in the Worlds tab, by the tab's name",
    (language) => {
      const hint = bundle(
        language,
        "notifications"
      )._trn_world_saved_as_copy_hint
      const tabs = bundle(language, "ui")
      expect(hint).toContain(tabs._trn_worlds)
      expect(hint).not.toContain(tabs._trn_addons)
    }
  )

  it("names the datapack and server addon tasks instead of the generic fallback", () => {
    expect(getTaskTranslationKey("InstanceTaskStageDatapack")).toBe(
      "tasks:_trn_instance_task_stage_datapack"
    )
    expect(getTaskTranslationKey("ServerTaskInstallMod")).toBe(
      "tasks:_trn_server_task_install_mod"
    )
  })

  it.each([
    ["InstanceImportJavaPathSkipped", "_trn_import_java_path_skipped"],
    ["InstanceImportCommandsSkipped", "_trn_import_commands_skipped"],
    ["InstanceImportGameDirMissing", "_trn_import_game_dir_missing"],
    ["InstanceImportConfigUnreadable", "_trn_import_config_unreadable"],
    [
      "InstanceImportUnknownMinecraftVersion",
      "_trn_import_unknown_minecraft_version"
    ]
  ])("shows %s with its own copy on the import row", (variant, key) => {
    expect(getImportErrorTranslationKey(variant)).toBe(`instances:${key}`)
  })

  it("explains an HMCL running directory it cannot locate instead of calling the settings unreadable", () => {
    expect(
      getImportErrorTranslationKey("InstanceImportHmclRelativeRunDir")
    ).toBe("instances:_trn_import_hmcl_relative_run_dir")
  })
})
