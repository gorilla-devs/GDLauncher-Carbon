import { Component, Show, For, createSignal, onMount } from "solid-js"
import { useTransContext } from "@gd/i18n"
import {
  Button,
  Badge,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsIndicator,
  Separator,
  Tooltip,
  TooltipTrigger,
  TooltipContent
} from "@gd/ui"
import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Mod as ModType } from "@gd/core_module/bindings"
import CopyIcon from "@/components/CopyIcon"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"
import { rspc } from "@/utils/rspcClient"
import { getModImageUrl } from "@/utils/instances"
import { useGDNavigate } from "@/managers/NavigationManager"

interface ModDetailsProps extends ModalProps {
  data: {
    mod: ModType
    instanceId: number
  }
}

const ModDetails: Component<ModDetailsProps> = (props) => {
  const [t] = useTransContext()
  const navigator = useGDNavigate()
  const modalsContext = useModal()
  const [selectedTab, setSelectedTab] = createSignal("overview")
  const [modDescription, setModDescription] = createSignal("")
  const [_isLoadingDescription, setIsLoadingDescription] = createSignal(false)

  const mod = () => props.data.mod
  const displayName = () => mod().metadata?.name || mod().filename
  const version = () =>
    mod().metadata?.version ||
    mod().curseforge?.version ||
    mod().modrinth?.version ||
    t("modals:_trn_mod_details.unknown")

  onMount(async () => {
    // Fetch mod description if available from platforms
    setIsLoadingDescription(true)
    try {
      if (mod().curseforge) {
        const result = rspc.createQuery(() => ({
          queryKey: [
            "modplatforms.curseforge.getMod",
            { modId: mod().curseforge!.project_id }
          ]
        }))
        if (result.data?.data.summary) {
          setModDescription(result.data.data.summary)
        }
      } else if (mod().modrinth) {
        const result = rspc.createQuery(() => ({
          queryKey: [
            "modplatforms.modrinth.getProject",
            mod().modrinth!.project_id
          ]
        }))
        if (result.data?.description) {
          setModDescription(result.data.description)
        }
      }
    } catch (error) {
      console.error("Failed to fetch mod description:", error)
    }
    setIsLoadingDescription(false)
  })

  const getModImage = () => {
    if (mod().curseforge?.has_image) {
      return getModImageUrl(
        props.data.instanceId.toString(),
        mod().id,
        "curseforge"
      )
    } else if (mod().modrinth?.has_image) {
      return getModImageUrl(
        props.data.instanceId.toString(),
        mod().id,
        "modrinth"
      )
    } else if (mod().metadata?.has_image) {
      return getModImageUrl(
        props.data.instanceId.toString(),
        mod().id,
        "metadata"
      )
    }
    return null
  }

  const tabs = () => [
    { value: "overview", label: t("modals:_trn_mod_details.overview") },
    { value: "technical", label: t("modals:_trn_mod_details.technical") }
  ]

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      height="h-[600px]"
      width="w-[900px]"
      class="max-w-[95vw]"
      scrollable="overflow-y-auto overflow-x-hidden"
    >
      <div class="flex flex-col gap-6">
        {/* Header Section */}
        <div class="flex gap-6">
          <Show
            when={getModImage()}
            fallback={
              <div class="bg-darkSlate-700 flex h-24 w-24 items-center justify-center rounded-lg shadow-lg">
                <div class="i-hugeicons:zip-01 text-lightSlate-600 text-4xl" />
              </div>
            }
          >
            <img
              src={getModImage()!}
              alt={displayName()}
              class="h-24 w-24 rounded-lg object-cover shadow-lg"
            />
          </Show>
          <div class="flex flex-1 flex-col gap-2">
            <div class="flex items-start justify-between">
              <div>
                <h2 class="flex items-center gap-2 text-2xl font-bold">
                  {displayName()}
                  <CopyIcon text={displayName()} />
                </h2>
                <Show when={mod().filename !== displayName()}>
                  <p class="text-lightSlate-600 text-sm">{mod().filename}</p>
                </Show>
              </div>
              <Badge variant={mod().enabled ? "success" : "secondary"}>
                {mod().enabled
                  ? t("instances:_trn_enabled")
                  : t("instances:_trn_disabled")}
              </Badge>
            </div>

            <div class="flex items-center gap-4 text-sm">
              <div class="flex items-center gap-2">
                <span class="text-lightSlate-600">
                  {t("modals:_trn_mod_details.version")}:
                </span>
                <span class="font-medium">{version()}</span>
              </div>

              <Show when={mod().curseforge || mod().modrinth}>
                <div class="flex items-center gap-2">
                  <span class="text-lightSlate-600">
                    {t("modals:_trn_mod_details.platform")}:
                  </span>
                  <div class="flex gap-1">
                    <Show when={mod().curseforge}>
                      <Tooltip>
                        <TooltipTrigger>
                          <img
                            src={CurseforgeLogo}
                            class="h-4 w-4"
                            alt={t("enums:_trn_curseforge")}
                          />
                        </TooltipTrigger>
                        <TooltipContent>
                          {t("enums:_trn_curseforge")}
                        </TooltipContent>
                      </Tooltip>
                    </Show>
                    <Show when={mod().modrinth}>
                      <Tooltip>
                        <TooltipTrigger>
                          <img
                            src={ModrinthLogo}
                            class="h-4 w-4"
                            alt={t("enums:_trn_modrinth")}
                          />
                        </TooltipTrigger>
                        <TooltipContent>
                          {t("enums:_trn_modrinth")}
                        </TooltipContent>
                      </Tooltip>
                    </Show>
                  </div>
                </div>
              </Show>

              <Show when={mod().has_update}>
                <Badge variant="warning" class="gap-1">
                  <div class="i-hugeicons:download-02" />
                  {t("modals:_trn_mod_details.update_available")}
                </Badge>
              </Show>
            </div>
          </div>
        </div>

        <Separator />

        {/* Tabs */}
        <Tabs value={selectedTab()} onChange={setSelectedTab}>
          <TabsList class="w-fit">
            <TabsIndicator />
            <For each={tabs()}>
              {(tab) => (
                <TabsTrigger value={tab.value}>{tab.label}</TabsTrigger>
              )}
            </For>
          </TabsList>
        </Tabs>

        {/* Tab Content */}
        <div class="min-h-[200px]">
          <Show when={selectedTab() === "overview"}>
            {/* Overview Tab */}
            <div class="space-y-4">
              <Show when={mod().metadata?.description || modDescription()}>
                <div>
                  <h3 class="mb-2 font-semibold">
                    {t("modals:_trn_mod_details.description")}
                  </h3>
                  <p class="text-lightSlate-300 text-sm">
                    {mod().metadata?.description ||
                      modDescription() ||
                      t("modals:_trn_mod_details.no_description")}
                  </p>
                </div>
              </Show>

              <Show when={mod().metadata?.authors}>
                <div>
                  <h3 class="mb-2 font-semibold">
                    {t("modals:_trn_mod_details.authors")}
                  </h3>
                  <p class="text-lightSlate-300 text-sm">
                    {mod().metadata!.authors}
                  </p>
                </div>
              </Show>

              <div class="flex flex-col gap-4">
                <Show when={mod().curseforge}>
                  <div class="flex flex-col gap-2">
                    <div class="flex items-center gap-2">
                      <img
                        src={CurseforgeLogo}
                        class="h-4 w-4"
                        alt="CurseForge"
                      />
                      <span class="text-sm font-semibold">
                        {t("content:_trn_view_on_curseforge")}
                      </span>
                    </div>
                    <div class="flex gap-2">
                      <Button
                        size="small"
                        type="secondary"
                        onClick={() => {
                          const projectId = mod().curseforge!.project_id
                          navigator.navigate(`/addon/${projectId}/curseforge`)
                          modalsContext?.closeModal()
                        }}
                      >
                        <div class="i-hugeicons:dashboard-square-01" />
                        {t("content:_trn_open_in_app")}
                      </Button>
                      <Button
                        size="small"
                        type="secondary"
                        onClick={() => {
                          const slug = mod().curseforge!.urlslug
                          window.open(
                            `https://www.curseforge.com/minecraft/mc-mods/${slug}`,
                            "_blank"
                          )
                        }}
                      >
                        <div class="i-hugeicons:link-square-02" />
                        {t("content:_trn_open_in_browser")}
                      </Button>
                    </div>
                  </div>
                </Show>
                <Show when={mod().modrinth}>
                  <div class="flex flex-col gap-2">
                    <div class="flex items-center gap-2">
                      <img src={ModrinthLogo} class="h-4 w-4" alt="Modrinth" />
                      <span class="text-sm font-semibold">
                        {t("content:_trn_view_on_modrinth")}
                      </span>
                    </div>
                    <div class="flex gap-2">
                      <Button
                        size="small"
                        type="secondary"
                        onClick={() => {
                          navigator.navigate(
                            `/addon/${mod().modrinth!.project_id}/modrinth`
                          )
                          modalsContext?.closeModal()
                        }}
                      >
                        <div class="i-hugeicons:dashboard-square-01" />
                        {t("content:_trn_open_in_app")}
                      </Button>
                      <Button
                        size="small"
                        type="secondary"
                        onClick={() => {
                          window.open(
                            `https://modrinth.com/mod/${mod().modrinth!.project_id}`,
                            "_blank"
                          )
                        }}
                      >
                        <div class="i-hugeicons:link-square-02" />
                        {t("content:_trn_open_in_browser")}
                      </Button>
                    </div>
                  </div>
                </Show>
              </div>
            </div>
          </Show>

          <Show when={selectedTab() === "technical"}>
            {/* Technical Tab */}
            <div class="space-y-3">
              <div class="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span class="text-lightSlate-600">
                    {t("modals:_trn_mod_details.file_id")}:
                  </span>
                  <div class="mt-1 flex items-center gap-2 font-mono">
                    {mod().id}
                    <CopyIcon text={mod().id} />
                  </div>
                </div>

                <Show when={mod().metadata?.modid}>
                  <div>
                    <span class="text-lightSlate-600">
                      {t("modals:_trn_mod_details.mod_id")}:
                    </span>
                    <div class="mt-1 flex items-center gap-2 font-mono">
                      {mod().metadata!.modid}
                      <CopyIcon text={mod().metadata!.modid} />
                    </div>
                  </div>
                </Show>

                <Show when={mod().curseforge}>
                  <div>
                    <span class="text-lightSlate-600">
                      {t("modals:_trn_mod_details.curseforge_project_id")}:
                    </span>
                    <div class="mt-1 flex items-center gap-2 font-mono">
                      {mod().curseforge!.project_id}
                      <CopyIcon
                        text={mod().curseforge!.project_id.toString()}
                      />
                    </div>
                  </div>
                  <div>
                    <span class="text-lightSlate-600">
                      {t("modals:_trn_mod_details.curseforge_file_id")}:
                    </span>
                    <div class="mt-1 flex items-center gap-2 font-mono">
                      {mod().curseforge!.file_id}
                      <CopyIcon text={mod().curseforge!.file_id.toString()} />
                    </div>
                  </div>
                </Show>

                <Show when={mod().modrinth}>
                  <div>
                    <span class="text-lightSlate-600">
                      {t("modals:_trn_mod_details.modrinth_project_id")}:
                    </span>
                    <div class="mt-1 flex items-center gap-2 font-mono">
                      {mod().modrinth!.project_id}
                      <CopyIcon text={mod().modrinth!.project_id} />
                    </div>
                  </div>
                  <div>
                    <span class="text-lightSlate-600">
                      {t("modals:_trn_mod_details.modrinth_version_id")}:
                    </span>
                    <div class="mt-1 flex items-center gap-2 font-mono">
                      {mod().modrinth!.version_id}
                      <CopyIcon text={mod().modrinth!.version_id} />
                    </div>
                  </div>
                </Show>
              </div>

              <Separator />

              <div>
                <h4 class="mb-2 font-semibold">
                  {t("modals:_trn_mod_details.file_hashes")}
                </h4>
                <div class="space-y-2 text-sm">
                  <Show when={mod().metadata?.sha_1}>
                    <div>
                      <span class="text-lightSlate-600">
                        {t("modals:_trn_mod_details.sha1")}:
                      </span>
                      <div class="mt-1 flex items-start gap-2 font-mono">
                        <span class="flex-1 break-all">
                          {mod().metadata!.sha_1}
                        </span>
                        <CopyIcon text={mod().metadata!.sha_1} />
                      </div>
                    </div>
                  </Show>
                  <Show when={mod().metadata?.sha_512}>
                    <div>
                      <span class="text-lightSlate-600">
                        {t("modals:_trn_mod_details.sha512")}:
                      </span>
                      <div class="mt-1 flex items-start gap-2 font-mono">
                        <span class="flex-1 break-all">
                          {mod().metadata!.sha_512}
                        </span>
                        <CopyIcon text={mod().metadata!.sha_512} />
                      </div>
                    </div>
                  </Show>
                  <Show when={mod().metadata?.murmur_2}>
                    <div>
                      <span class="text-lightSlate-600">
                        {t("modals:_trn_mod_details.murmur2_signed")}:
                      </span>
                      <div class="mt-1 flex items-start gap-2 font-mono">
                        <span class="flex-1 break-all">
                          {mod().metadata!.murmur_2}
                        </span>
                        <CopyIcon text={mod().metadata!.murmur_2} />
                      </div>
                    </div>
                    <div>
                      <span class="text-lightSlate-600">
                        {t("modals:_trn_mod_details.murmur2_unsigned")}:
                      </span>
                      <div class="mt-1 flex items-start gap-2 font-mono">
                        <span class="flex-1 break-all">
                          {(
                            parseInt(mod().metadata!.murmur_2) >>> 0
                          ).toString()}
                        </span>
                        <CopyIcon
                          text={(
                            parseInt(mod().metadata!.murmur_2) >>> 0
                          ).toString()}
                        />
                      </div>
                    </div>
                  </Show>
                </div>
              </div>

              <Show when={(mod().metadata?.modloaders?.length || 0) > 0}>
                <div>
                  <h4 class="mb-2 font-semibold">
                    {t("modals:_trn_mod_details.supported_modloaders")}
                  </h4>
                  <div class="flex gap-2">
                    <For each={mod().metadata?.modloaders || []}>
                      {(loader) => <Badge variant="secondary">{loader}</Badge>}
                    </For>
                  </div>
                </div>
              </Show>
            </div>
          </Show>
        </div>
      </div>
    </ModalLayout>
  )
}

export default ModDetails
