import { extractErrorCode, queryClient, rspc } from "@/utils/rspcClient"
import { i18n, Trans, useTransContext } from "@gd/i18n"
import { createMemo, For } from "solid-js"
import { Button, toast } from "@gd/ui"
import { getBuiltinTheme } from "@/themes/builtin"
import { themeRefusalKey } from "@/themes/errors"
import { installedTileModel } from "@/themes/installed"
import { THEME_EXTENSION } from "@/themes/install"
import { useModal } from "@/managers/ModalsManager"
import useSettingsGeneralData from "./settings.general.data"
import LoadingError from "@/components/LoadingError"
import { getAvailableThemes } from "@/utils/theme"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"
import BuiltinThemeTile from "./appearance/BuiltinThemeTile"
import InstalledThemeTile from "./appearance/InstalledThemeTile"
import CustomizeTheme from "./appearance/CustomizeTheme"

const Appearance = () => {
  const routeData = useSettingsGeneralData()
  const modals = useModal()
  const [t] = useTransContext()
  const themeName = () => routeData?.data?.data?.theme || "main"

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"],
    onMutate: (newTheme) => {
      queryClient.setQueryData(["settings.setSettings"], newTheme)
    }
  }))
  const select = (key: string) => {
    settingsMutation.mutate({ theme: { Set: key } })
  }
  const openFolder = rspc.createMutation(() => ({
    mutationKey: ["themes.openFolder"],
    // A refused folder (a link, or one deleted meanwhile) comes back with a
    // THEME_* code, which the global handler only logs.
    onError: (error) => {
      const key = themeRefusalKey(extractErrorCode(error))
      if (key) toast.error(t(key))
    }
  }))
  const pickThemeFile = async () => {
    const result = await window.openFileDialog({
      properties: ["openFile"],
      filters: [
        {
          name: t("settings:_trn_theme_file_filter"),
          extensions: [THEME_EXTENSION.slice(1)]
        }
      ]
    })
    const path = result.filePaths?.[0]
    if (result.canceled || !path) return
    modals?.openModal({ name: "themeInstall" }, { path })
  }

  const themes = getAvailableThemes()

  const season = rspc.createQuery(() => ({ queryKey: ["themes.getSeason"] }))
  // Reconciled by id, so a refresh updates each tile in place instead of
  // remounting it (which reloads its preview and closes its tooltips).
  const installed = rspc.createQuery(() => ({
    queryKey: ["themes.list"],
    reconcile: "id"
  }))
  const language = () => i18n.language || "english"
  const mainColors = getBuiltinTheme("main").colors

  return (
    <LoadingError routeData={routeData}>
      <div class="w-full">
        <PageTitle>
          <Trans key="settings:_trn_appearance" />
        </PageTitle>
        <Row forceContentBelow class="border-box w-full max-w-full">
          <Title
            description={<Trans key="settings:_trn_default_themes_text" />}
          >
            <Trans key="settings:_trn_default_themes_title" />
          </Title>
          <div class="flex w-full flex-wrap gap-6">
            <For each={themes}>
              {(theme) => (
                <BuiltinThemeTile
                  theme={theme}
                  savedKey={themeName()}
                  season={season.data}
                  onSelect={select}
                />
              )}
            </For>
          </div>
        </Row>
        <Row forceContentBelow class="border-box w-full max-w-full">
          <div class="flex items-start justify-between gap-4">
            <Title
              description={<Trans key="settings:_trn_custom_themes_text" />}
            >
              <Trans key="settings:_trn_custom_themes_title" />
            </Title>
            <div class="mt-2 flex shrink-0 items-center gap-2">
              <Button
                type="primary"
                data-testid="theme-install"
                onClick={() => void pickThemeFile()}
              >
                <Trans key="settings:_trn_install_theme" />
              </Button>
              <Button
                type="text"
                data-testid="themes-open-folder"
                onClick={() => openFolder.mutate({ id: null })}
              >
                <Trans key="settings:_trn_open_themes_folder" />
              </Button>
            </div>
          </div>
          <div class="flex w-full flex-wrap gap-6">
            <For each={installed.data ?? []}>
              {(theme) => {
                const model = createMemo(() =>
                  installedTileModel(theme, themeName(), language(), mainColors)
                )
                return (
                  <InstalledThemeTile
                    id={theme.id}
                    model={model()}
                    savedKey={themeName()}
                    onSelect={select}
                    onOpenFolder={() =>
                      openFolder.mutate({
                        id: model().removable ? theme.id : null
                      })
                    }
                    onRemove={() =>
                      modals?.openModal(
                        { name: "themeRemove" },
                        { id: theme.id, name: model().name }
                      )
                    }
                  />
                )
              }}
            </For>
          </div>
        </Row>
        <CustomizeTheme />
      </div>
    </LoadingError>
  )
}

export default Appearance
