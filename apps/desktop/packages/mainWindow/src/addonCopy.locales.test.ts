import { readFileSync } from "node:fs"
import { describe, expect, it } from "vitest"
import { createInstance } from "i18next"
import { createI18nOptions, loadLanguageFiles } from "@gd/i18n"
import {
  getImportErrorTranslationKey,
  getTaskTranslationKey
} from "../../../../../packages/i18n/helpers"

// The addon, datapack, search, login, cache and updater copy added by the
// fix round shipped in English only. Every key below is shown to players, so
// each of the seven shipped languages must carry its own text.
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
    "_trn_datapack_staging.title",
    "_trn_datapack_staging.description",
    "_trn_datapack_staging.pack_label",
    "_trn_datapack_staging.choose_pack",
    "_trn_datapack_staging.world_label",
    "_trn_datapack_staging.choose_world",
    "_trn_datapack_staging.add_to_world",
    "_trn_datapack_staging.refresh_worlds",
    "_trn_datapack_staging.no_worlds",
    "_trn_datapack_staging.added",
    "_trn_datapack_row_subtitle",
    "_trn_datapack_toggle_hint",
    "_trn_datapack_update_hint",
    "_trn_datapack_delete_hint"
  ],
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
    "_trn_curseforge_file_invalid"
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
    "_trn_confirm_world_deletion_title_plural",
    "_trn_confirm_world_deletion_body",
    "_trn_confirm_world_deletion_body_plural",
    "_trn_confirm_world_deletion_body_selection",
    "_trn_confirm_world_deletion_dont_ask",
    "_trn_confirm_world_deletion_cancel",
    "_trn_confirm_world_deletion_confirm",
    "_trn_confirm_world_deletion_confirm_plural",
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
    "_trn_instance_task_check_pack_origin_versions"
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

describe("fix-round addon and account copy is translated", () => {
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
