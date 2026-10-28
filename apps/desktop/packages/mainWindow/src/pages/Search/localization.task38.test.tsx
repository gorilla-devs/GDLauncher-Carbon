import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { createSignal, For } from "solid-js"
import { render } from "solid-js/web"
import { createInstance } from "i18next"
import {
  createI18nOptions,
  loadLanguageFiles,
  supportedLanguages,
  TransProvider,
  Trans,
  useTransContext
} from "@gd/i18n"
import { GdlAccountStep } from "../Login/steps/GdlAccountStep"
import { CompleteStep } from "../Login/components/CompleteStep"
import SummaryStep from "@/managers/ModalsManager/modals/DuplicatedModsResolution/SummaryStep"
import { worldVersionCopy } from "@/components/ModDownloadButton/worldVersionCopy"
import { Grid } from "./Grid"
import { List } from "./List"
import SortDisplay from "./FiltersDisplay/SortDisplay"
import { SortFilter } from "./FilterSidebar/SortFilter"
import ConfirmWorldDeletion from "@/managers/ModalsManager/modals/ConfirmWorldDeletion"
import { getEmptyAddonPhraseKey } from "./translationKeys"
import type { FEUnifiedSearchType } from "@gd/core_module/bindings"

const c = vi.hoisted(() => ({
  context: undefined as any,
  params: { type: "mod" },
  effects: [] as string[],
  save: undefined as any
}))
vi.mock("../Login/flow/FlowContext", () => ({
  useFlow: () => ({
    data: { accounts: [], activeUuid: "synthetic", gdlAccountId: "synthetic" }
  })
}))
vi.mock("@/utils/instances", () => ({
  getModImageUrl: () => {
    throw new Error("Image/provider route is excluded")
  }
}))
vi.mock("@/components/SearchInputContext", () => ({ default: () => c.context }))
vi.mock("@solidjs/router", () => ({
  useParams: () => c.params,
  useLocation: () => ({ search: "" })
}))
vi.mock("@/managers/NavigationManager", () => ({
  useGDNavigate: () => ({ navigate: vi.fn() })
}))
vi.mock("@/utils/rspcClient", () => ({
  rspc: {
    createQuery: () => ({ data: [] }),
    createMutation: () => ({
      mutateAsync: async () => {
        c.effects.push("save")
        await c.save
        c.effects.push("saved")
      }
    })
  }
}))
vi.mock("@/components/PlaceholderGorilla", () => ({
  PlaceholderGorilla: () => <span data-gorilla />
}))
vi.mock("./GridItem", () => ({ GridItem: () => <div data-result>result</div> }))
vi.mock("./ListItem", () => ({ ListItem: () => <div data-result>result</div> }))
vi.mock("@/components/SharePreviewContent", () => ({ default: () => <div /> }))
vi.mock("@/components/VirtuaWrapper", () => ({
  VList: (p: any) => (
    <div>
      <For each={p.data}>{p.children}</For>
    </div>
  )
}))
vi.mock("@/managers/ModalsManager", () => ({
  useModal: () => ({ closeModal: () => c.effects.push("close") })
}))
vi.mock("@/managers/ModalsManager/ModalLayout", () => ({
  default: (p: any) => <div>{p.children}</div>
}))
vi.mock("@gd/ui", async () => {
  const { Radio } = await import("../../../../../../../packages/ui/src/Radio")
  const Skeleton = Object.assign(() => <div data-loading>loading</div>, {
    searchList: () => <div data-loading>loading</div>
  })
  return {
    Radio,
    Skeleton,
    Collapsable: (p: any) => (
      <section>
        <header>{p.title}</header>
        {p.children}
      </section>
    ),
    Badge: (p: any) => (
      <button data-badge onClick={p.onClick}>
        {p.children}
      </button>
    ),
    Button: (p: any) => (
      <button data-testid={p["data-testid"]} onClick={p.onClick}>
        {p.children}
      </button>
    ),
    Checkbox: (p: any) => (
      <label>
        <input
          type="checkbox"
          checked={p.checked}
          onChange={(e) => p.onChange(e.currentTarget.checked)}
        />
        {p.children}
      </label>
    )
  }
})
const dispose: (() => void)[] = []
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
afterEach(() => {
  dispose.splice(0).forEach((d) => d())
  document.body.textContent = ""
  c.effects = []
  c.save = undefined
})
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    }
  )
})
function context(mode = "grid", type = "mod") {
  const [query, setQuery] = createSignal<any>({
    projectType: type,
    searchApi: "modrinth",
    platformFilters: null
  })
  const [loading, setLoading] = createSignal(false)
  const [rows, setRows] = createSignal<any[]>([])
  c.params.type = type
  c.context = {
    searchQuery: query,
    setSearchQuery: setQuery,
    allRows: rows,
    isInitialLoading: loading,
    viewMode: () => mode,
    selectedInstanceId: () => NaN,
    selectedServerId: () => NaN,
    defaultAddonType: () => "modpack",
    directErrors: () => [],
    isShareMode: () => false,
    shareCode: () => undefined,
    ref: () => undefined,
    setRef: vi.fn(),
    hasNextPage: () => false
  }
  return { query, setQuery, setLoading, setRows }
}
async function mount(
  id: string,
  child: () => any,
  overrides?: Record<string, Record<string, string>>
) {
  const english = await loadLanguageFiles("english"),
    selected = await loadLanguageFiles(id)
  if (overrides) {
    for (const [ns, values] of Object.entries(overrides))
      selected[ns] = { ...selected[ns], ...values }
  }
  const instance = createInstance()
  const options = createI18nOptions(id, english, selected)
  await instance.init(options)
  const root = document.createElement("div")
  document.body.append(root)
  dispose.push(
    render(
      () => (
        <TransProvider instance={instance} options={options}>
          {child()}
        </TransProvider>
      ),
      root
    )
  )
  await flush()
  return { root, instance, selected }
}
const ids = Object.keys(supportedLanguages)
const localizedPhrases: Record<string, string[]> = {
  italian: [
    "mod",
    "modpack",
    "pacchetti di risorse",
    "shader",
    "mondi",
    "plugin",
    "pacchetti di dati",
    "contenuti aggiuntivi"
  ],
  japanese: [
    "MOD",
    "MODパック",
    "リソースパック",
    "シェーダー",
    "ワールド",
    "プラグイン",
    "データパック",
    "アドオン"
  ],
  russian: [
    "моды",
    "сборки модов",
    "пакеты ресурсов",
    "шейдеры",
    "миры",
    "плагины",
    "пакеты данных",
    "дополнения"
  ],
  spanish: [
    "mods",
    "modpacks",
    "paquetes de recursos",
    "shaders",
    "mundos",
    "plugins",
    "paquetes de datos",
    "complementos"
  ],
  simplified_chinese: [
    "模组",
    "整合包",
    "资源包",
    "光影",
    "世界",
    "插件",
    "数据包",
    "附加内容"
  ],
  french: [
    "mods",
    "packs de mods",
    "packs de ressources",
    "shaders",
    "mondes",
    "plugins",
    "packs de données",
    "extensions"
  ]
}
const englishPhrases: Record<string, string> = {
  mod: "mods",
  modpack: "modpacks",
  resourcePack: "resource packs",
  shader: "shaders",
  world: "worlds",
  plugin: "plugins",
  datapack: "data packs",
  unknown: "addons"
}
it.each(
  ids.flatMap((id) =>
    ["grid", "list"].flatMap((mode) =>
      Object.keys(englishPhrases)
        .filter((type) => type !== "plugin" && type !== "unknown")
        .map((type) => [id, mode, type])
    )
  )
)("GPT0184 %s %s empty %s uses localized phrases", async (id, mode, type) => {
  context(mode, type)
  const { root } = await mount(id, () =>
    mode === "grid" ? <Grid /> : <List />
  )
  const text = root.querySelector("p")!.textContent
  expect(text).not.toMatch(/\{type\}|_trn_/)
  if (id === "english")
    expect(text).toContain(`any ${englishPhrases[type]} matching`)
  else {
    expect(text).not.toContain("resourcePack")
    expect(text).toContain(
      localizedPhrases[id][Object.keys(englishPhrases).indexOf(type)]
    )
  }
})
// The backend's plugin/unknown phrase keys remain useful without making them
// browsable routes. Deliver them through the same plain Trans text sink.
const EmptyPhrase = (props: { type: string }) => {
  const [t] = useTransContext()
  return (
    <p>
      <Trans
        key="search:_trn_no_results_description"
        options={{
          type: t(getEmptyAddonPhraseKey(props.type as FEUnifiedSearchType))
        }}
      />
    </p>
  )
}
it.each(ids.flatMap((id) => ["plugin", "unknown"].map((type) => [id, type])))(
  "GPT0184 %s non-route %s phrase remains localized",
  async (id, type) => {
    const { root } = await mount(id, () => <EmptyPhrase type={type} />)
    const text = root.querySelector("p")!.textContent
    expect(text).not.toMatch(/\{type\}|_trn_/)
    if (id === "english")
      expect(text).toContain(`any ${englishPhrases[type]} matching`)
    else
      expect(text).toContain(
        localizedPhrases[id][Object.keys(englishPhrases).indexOf(type)]
      )
  }
)
it.each(["grid", "list"])(
  "GPT0184 %s initial loading hides empty; nonempty rows stay visible",
  async (mode) => {
    const state = context(mode)
    state.setLoading(true)
    const { root } = await mount("english", () =>
      mode === "grid" ? <Grid /> : <List />
    )
    expect(root.querySelector("p")).toBeNull()
    expect(root.querySelector("[data-loading]")).not.toBeNull()
    state.setLoading(false)
    await flush()
    expect(root.querySelector("p")?.textContent).toContain("any mods matching")
    state.setRows([
      { type: "value", value: { id: "synthetic", platform: "modrinth" } }
    ])
    await flush()
    expect(root.querySelector("p")).toBeNull()
    expect(root.querySelector("[data-result]")).not.toBeNull()
  }
)
const sorts = [
  ...[
    "featured",
    "popularity",
    "totalDownloads",
    "lastUpdated",
    "name",
    "author"
  ].map((value) => ["curseforge", value]),
  ...["relevance", "downloads", "follows", "newest", "updated"].map((value) => [
    "modrinth",
    value
  ])
]
const sortKeys: Record<string, string> = {
  totalDownloads: "downloads",
  lastUpdated: "last_updated",
  featured: "featured",
  popularity: "popularity",
  name: "name",
  author: "author",
  relevance: "relevance",
  downloads: "downloads",
  follows: "follows",
  newest: "newest",
  updated: "updated"
}
it.each(
  ids.flatMap((id) => sorts.map(([platform, value]) => [id, platform, value]))
)(
  "GPT0268 %s %s %s badge includes localized selected label; heading/checked/clear remain",
  async (id, platform, value) => {
    const state = context()
    state.setQuery({
      projectType: "mod",
      searchApi: platform,
      platformFilters: {
        platform,
        filters:
          platform === "curseforge"
            ? { sort_field: value, sort_order: null }
            : { sort_index: value }
      }
    })
    const { root, instance } = await mount(id, () => (
      <>
        <SortFilter />
        <SortDisplay />
      </>
    ))
    const label = instance.t(`search:_trn_${sortKeys[value]}`)
    const badge = root.querySelector<HTMLButtonElement>("[data-badge]")!
    expect(badge.textContent).toContain(label)
    expect(badge.textContent).not.toBe(instance.t("search:_trn_sort_by"))
    expect(root.querySelector("header")?.textContent).toContain(
      instance.t("search:_trn_sort_by")
    )
    expect(
      root.querySelector<HTMLInputElement>(`input[value="${value}"]`)?.checked
    ).toBe(true)
    badge.click()
    expect(state.query().platformFilters).toBeNull()
  }
)
it("GPT0268 sort-order interpolation remains visible and radio deselection retains filter semantics", async () => {
  const state = context()
  state.setQuery({
    projectType: "mod",
    searchApi: "curseforge",
    platformFilters: {
      platform: "curseforge",
      filters: { sort_field: "name", sort_order: "ascending" }
    }
  })
  const { root } = await mount("english", () => (
    <>
      <SortFilter />
      <SortDisplay />
    </>
  ))
  expect(root.textContent).toContain("Order: Ascending")
  root.querySelector<HTMLInputElement>('input[value="name"]')!.click()
  expect(state.query().platformFilters.filters.sort_field).toBeNull()
  expect(state.query().platformFilters.filters.sort_order).toBe("ascending")
})
it.each(
  ids.flatMap((id) =>
    ["Bob's & Friends", '<b>& "name"</b>', "Synthetic World"].map((name) => [
      id,
      name
    ])
  )
)("M17 %s singular warning displays literal filename %j", async (id, name) => {
  const { root, instance } = await mount(id, () => (
    <ConfirmWorldDeletion
      title="Synthetic confirmation"
      data={{ worldName: name, onConfirm: () => c.effects.push("confirm") }}
    />
  ))
  expect(root.textContent).toContain(name)
  expect(root.querySelector("b")).toBeNull()
  expect(
    instance.t("instances:_trn_confirm_world_deletion_body", { name })
  ).toContain(name === "Synthetic World" ? name : "&")
  if (name.includes("<b>"))
    expect(
      instance.t("instances:_trn_confirm_world_deletion_body", { name })
    ).toContain("&lt;b&gt;")
})
it.each(ids)(
  "PB3 %s fallback plural/mixed warnings and missing import/repair keys resolve without markers",
  async (id) => {
    const { root, instance } = await mount(id, () => (
      <ConfirmWorldDeletion
        title="Synthetic confirmation"
        data={{
          worldCount: 2,
          totalCount: 4,
          onConfirm: () => c.effects.push("confirm")
        }}
      />
    ))
    expect(root.textContent).not.toMatch(/UNTRANSLATED|_trn_/)
    expect(root.textContent).toContain("2")
    expect(root.textContent).toContain("4")
    // Translated in every shipped language now, not an English fallback.
    const importError = instance.t("instances:_trn_import_error_bad_config")
    expect(importError).not.toContain("_trn_")
    expect(importError).not.toBe("")
    if (id !== "english")
      expect(importError).not.toBe("Configuration missing or unreadable")
    expect(instance.t("instances:_trn_repair_origin_incomplete")).not.toContain(
      "_trn_"
    )
  }
)
it("M17 warning preference saves before actual callback; Cancel performs no deletion", async () => {
  let finish!: () => void
  c.save = new Promise<void>((resolve) => (finish = resolve))
  const { root } = await mount("english", () => (
    <ConfirmWorldDeletion
      title="Synthetic confirmation"
      data={{
        worldName: "Synthetic World",
        onConfirm: () => c.effects.push("confirm")
      }}
    />
  ))
  root.querySelector<HTMLInputElement>('input[type="checkbox"]')!.click()
  root
    .querySelector<HTMLButtonElement>('[data-testid="world-deletion-confirm"]')!
    .click()
  await flush()
  expect(c.effects).toEqual(["save"])
  finish()
  await flush()
  expect(c.effects).toEqual(["save", "saved", "confirm", "close"])
  c.effects = []
  ;[...root.querySelectorAll("button")]
    .find((b) => b.textContent === "Cancel")!
    .click()
  expect(c.effects).toEqual(["close"])
})

it.each(["not-a-type", "__proto__", "toString"])(
  "GPT0184 non-route %s retains generic phrase delivery",
  async (type) => {
    const { root } = await mount("english", () => <EmptyPhrase type={type} />)
    expect(root.querySelector("p")?.textContent).toContain(
      "any addons matching"
    )
  }
)
it.each(ids)(
  "GPT0266 %s found/linked/setup/completion UI preserves localized accurate account copy",
  async (id) => {
    const link = vi.fn(),
      setup = vi.fn()
    const { root, instance } = await mount(id, () => (
      <>
        <GdlAccountStep
          step={{
            type: "gdl-account",
            gdlAccount: {
              type: "found-existing",
              data: { displayName: "Synthetic User" } as any
            }
          }}
        />
        <GdlAccountStep
          step={{ type: "gdl-account", gdlAccount: { type: "linked" } }}
        />
        <CompleteStep hasGDLAccount onContinue={() => {}} />
        <CompleteStep
          foundExistingAccount
          foundGDLAccountData={{
            displayName: "Synthetic User",
            email: "synthetic@example.invalid",
            profileIconUrl: ""
          }}
          onContinue={() => {}}
          onLinkExistingAccount={link}
        />
        <CompleteStep onContinue={() => {}} onSetupGDLAccount={setup} />
      </>
    ))
    expect(root.textContent).toContain("Synthetic User")
    expect(root.textContent).not.toMatch(/_trn_|\{\{name\}\}/)
    expect(root.textContent).toContain(
      instance.t("auth:_trn_login.cloud_sync_active")
    )
    const buttons = [...root.querySelectorAll("button")]
    buttons
      .find((b) =>
        b.textContent?.includes(instance.t("auth:_trn_login.sync_account"))
      )!
      .click()
    buttons
      .find((b) =>
        b.textContent?.includes(instance.t("auth:_trn_login.enable_cloud_sync"))
      )!
      .click()
    expect(link).toHaveBeenCalledTimes(1)
    expect(setup).toHaveBeenCalledTimes(1)
    if (id === "english") {
      expect(root.textContent).not.toMatch(
        /restore|sync is active|Settings sync|across all your devices|instance backups/i
      )
      expect(root.textContent).toContain("GDLauncher account linked")
    }
  }
)
it.each(["disable", "remove"] as const)(
  "GPT0138 Italian summary distinctly translates %s while keep/title/Apply remain",
  async (action) => {
    const { root } = await mount("italian", () => (
      <SummaryStep
        mods={[
          {
            groupKey: "synthetic",
            name: "Synthetic Mod",
            versions: [
              {
                id: "keep",
                fileName: "keep.jar",
                version: "1",
                dateAdded: "synthetic",
                fileSize: "1"
              },
              {
                id: "other",
                fileName: "other.jar",
                version: "2",
                dateAdded: "synthetic",
                fileSize: "2"
              }
            ]
          }
        ]}
        selectedVersions={{ synthetic: "keep" }}
        action={action}
        prevStep={() => {}}
        onFinish={() => {}}
        isApplying={false}
        instanceId={1}
      />
    ))
    expect(root.textContent).toContain("Rivedi Modifiche")
    expect(root.textContent).toContain("Applica Modifiche")
    expect(root.textContent).toContain("CONSERVA")
    expect(root.textContent).toContain(
      action === "disable" ? "DISABILITA" : "RIMUOVI"
    )
    expect(root.textContent).toContain(
      action === "disable"
        ? "Disabilita le versioni duplicate"
        : "Rimuovi le versioni duplicate"
    )
  }
)
it("M17 unescaped interpolation stays scoped to plain world text and default rich-child translation remains safe", async () => {
  const name = '<b>& "name"</b>'
  const { root, instance } = await mount("english", () => (
    <>
      <ConfirmWorldDeletion
        title="Synthetic confirmation"
        data={{ worldName: name }}
      />
      <Trans
        key="auth:_trn_login.how_does_it_work_text"
        options={{ account_id: name }}
      >
        {""}
        <span data-rich />
        {""}
      </Trans>
    </>
  ))
  expect(root.textContent).toContain(name)
  expect(root.querySelector("b")).toBeNull()
  expect(root.querySelector("[data-rich]")).not.toBeNull()
  expect(instance.options.interpolation?.escapeValue).toBe(true)
})

it("M17 a supplied localized singular-world resource remains literal at the same text sink", async () => {
  const name = "<b>Bob's & Friends</b>"
  const { root } = await mount(
    "italian",
    () => (
      <ConfirmWorldDeletion
        title="Synthetic confirmation"
        data={{ worldName: name }}
      />
    ),
    {
      instances: {
        _trn_confirm_world_deletion_body:
          '"{{name}}" sarà eliminato definitivamente. Questa operazione non può essere annullata.'
      }
    }
  )
  expect(root.textContent).toContain(`"${name}" sarà eliminato definitivamente`)
  expect(root.querySelector("b")).toBeNull()
})
it("GPT0137 actual provider registration/switch keeps grid empty copy reactive across selected locales", async () => {
  context()
  let actions!: ReturnType<typeof useTransContext>[1]
  const Child = () => {
    actions = useTransContext()[1]
    return <Grid />
  }
  const { root } = await mount("english", () => <Child />)
  expect(root.querySelector("p")?.textContent).toContain("any mods matching")
  const resources = await loadLanguageFiles("italian")
  for (const [ns, values] of Object.entries(resources))
    actions.addResources("it", ns, values)
  await actions.changeLanguage("it")
  await flush()
  expect(root.querySelector("p")?.textContent).toContain(
    "Nessun risultato per mod"
  )
  await actions.changeLanguage("en")
  await flush()
  expect(root.querySelector("p")?.textContent).toContain("any mods matching")
})

const deferredKeys = [
  "content:_trn_duplicates.retry_before_action",
  "content:_trn_duplicates.removed_cannot_disable",
  "content:_trn_duplicates.attempted_cannot_keep",
  "content:_trn_mods_updater.refresh_error",
  "java:_trn_runtime_recovery.keep_folders",
  "java:_trn_runtime_recovery.unresolved",
  "java:_trn_runtime_recovery.retry",
  "java:_trn_runtime_recovery.restart",
  "content:_trn_shader_loader_context_changed",
  "content:_trn_shader_loader_verification_failed",
  "content:_trn_shader_loader_install_failed",
  "content:_trn_shader_loader_uncertain",
  "instances:_trn_repair_preview_failed",
  "instances:_trn_repair_origin_failed",
  "instances:_trn_repair_origin_unavailable",
  "instances:_trn_repair_origin_read_failed",
  "notifications:_trn_shader_setup_closed_work_may_finish",
  "instances:_trn_batch_server_folder_unlink_confirmation",
  "instances:_trn_batch_folder_total_servers",
  "instances:_trn_server_singular",
  "instances:_trn_servers",
  "instances:_trn_folder_deletion_partial",
  "instances:_trn_deleting_server_selection_progress",
  "news:_trn_no_content",
  "news:_trn_patch_content_error",
  "instances:_trn_export_failed",
  "tasks:_trn_instance_task_install_world_copy"
] as const
it.each(ids)(
  "D8 %s actual Trans renders every deferred recovery/folder/news/export/world/shader consent key with preserved placeholders",
  async (id) => {
    const copy = worldVersionCopy(
      "world",
      "synthetic-version",
      "synthetic-owned-world"
    )!
    const { root } = await mount(id, () => (
      <>
        <For
          each={
            [
              ...deferredKeys,
              copy.action,
              copy.disclosure,
              copy.completion,
              "content:_trn_shader_loader_body_iris",
              "content:_trn_shader_loader_body_oculus",
              "content:_trn_shader_loader_body_vanilla"
            ] as const
          }
        >
          {(key) => (
            <p data-handoff>
              <Trans
                key={key}
                options={{
                  count: 4,
                  current: 2,
                  total: 5,
                  completed: 3,
                  world_name: "Synthetic World",
                  instance_name: "Synthetic Instance"
                }}
              />
            </p>
          )}
        </For>
      </>
    ))
    const rows = [...root.querySelectorAll("[data-handoff]")]
    expect(rows).toHaveLength(33)
    expect(root.textContent).not.toMatch(/UNTRANSLATED|_trn_|\{\{/)
    expect(root.textContent).toContain("Synthetic World")
    expect(root.textContent).toContain("Synthetic Instance")
    for (const token of [
      "4",
      "2",
      "5",
      "3",
      "Iris",
      "Sodium",
      "Oculus",
      "Embeddium",
      "Fabric"
    ])
      expect(root.textContent).toContain(token)
    expect(
      worldVersionCopy("world", "synthetic-version", "synthetic-owned-world", 1)
    ).toBeUndefined()
    expect(
      worldVersionCopy("mod", "synthetic-version", "synthetic-owned-world")
    ).toBeUndefined()
  }
)
