import { Button, Input, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import { Trans } from "@gd/i18n"
import PageTitle from "./components/PageTitle"
import RowsContainer from "./components/RowsContainer"
import Row from "./components/Row"
import Title from "./components/Title"
import {
  createEffect,
  createResource,
  createSignal,
  Match,
  onCleanup,
  Switch
} from "solid-js"
import Center from "./components/Center"
import { useModal } from "@/managers/ModalsManager"
import { isChangingRuntimePath } from "@/managers/ModalsManager/modals/ConfirmChangeRuntimePath/state"
import type { RuntimeTargetRefusal } from "../../../../main/utils/runtimePathGuard"

// Why a folder was refused, with what to pick instead.
const refusalTooltip = {
  not_absolute: "ui:_trn_tooltip.rtp_not_absolute",
  network_path: "ui:_trn_tooltip.rtp_network_path",
  filesystem_root: "ui:_trn_tooltip.rtp_drive_root",
  nested: "ui:_trn_tooltip.rtp_nested",
  install_overlap: "ui:_trn_tooltip.rtp_install_overlap",
  system_dir: "ui:_trn_tooltip.rtp_system_dir",
  removable_not_mounted: "ui:_trn_tooltip.rtp_not_mounted",
  not_a_folder: "ui:_trn_tooltip.rtp_not_a_folder",
  not_writable: "ui:_trn_tooltip.rtp_not_writable"
} as const satisfies Record<RuntimeTargetRefusal, string>
const refusalKey = (classification: string | undefined) =>
  classification && Object.hasOwn(refusalTooltip, classification)
    ? refusalTooltip[classification as RuntimeTargetRefusal]
    : undefined

const RuntimePath = () => {
  const modalsContext = useModal()

  const [runtimePath, setRuntimePath] = createSignal<string | undefined>(
    undefined
  )

  const [initialRuntimePath] = createResource(() => {
    return window.getInitialRuntimePath()
  })

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath()
  })

  createEffect(() => {
    if (currentRuntimePath() === undefined) {
      return
    }

    setRuntimePath(currentRuntimePath()!)
  })

  const [isPathValid, { refetch: revalidate }] = createResource(
    () => runtimePath() || false,
    async (path) => {
      return { path, classification: await window.validateRuntimePath(path) }
    }
  )

  const currentValidation = () => {
    // A resource retains its previous value while refreshing. Only a settled,
    // path-qualified result can authorize this selection.
    if (isPathValid.state !== "ready") return undefined
    const result = isPathValid()
    return result?.path === runtimePath() ? result : undefined
  }
  const canApply = () => {
    const result = currentValidation()
    return (
      !!result &&
      (result.classification === "valid" ||
        result.classification === "potentially_valid") &&
      !isChangingRuntimePath() &&
      currentRuntimePath() !== result.path
    )
  }

  // A refused folder can change outside the launcher (the user deletes a partial
  // copy in Explorer or Finder), so check it again when the window regains focus.
  // An accepted one is left alone, so a click that focuses the window still applies.
  const revalidateRefused = () => {
    if (runtimePath() === currentRuntimePath()) return
    const classification = currentValidation()?.classification
    if (
      isPathValid.state === "errored" ||
      (classification !== undefined &&
        classification !== "valid" &&
        classification !== "potentially_valid")
    )
      revalidate()
  }
  window.addEventListener("focus", revalidateRefused)
  onCleanup(() => window.removeEventListener("focus", revalidateRefused))

  return (
    <>
      <PageTitle>
        <Trans key="java:_trn_runtime_path" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title description={<Trans key="java:_trn_runtime_path_text" />}>
            <Trans key="java:_trn_runtime_path_title" />
          </Title>
          <Center>
            <Input
              class="w-full"
              value={runtimePath()}
              icon={
                <div
                  class="i-hugeicons:folder-01 hover:text-lightSlate-700 h-5 w-5 cursor-pointer transition-colors ease-spring"
                  onClick={async () => {
                    const result = await window.openFileDialog({
                      title: "Select Runtime Path",
                      defaultPath: runtimePath(),
                      properties: ["openDirectory"]
                    })

                    if (result.canceled) {
                      return
                    }

                    // Picking the same folder again is a request to check it again.
                    if (result.filePaths[0] === runtimePath()) revalidate()
                    else setRuntimePath(result.filePaths[0])
                  }}
                />
              }
              onInput={({ target: { value } }) => {
                setRuntimePath(value)
              }}
            />
            <Tooltip>
              <TooltipTrigger>
                <Button
                  rounded={false}
                  type="secondary"
                  class="h-10"
                  size="small"
                  onClick={() => {
                    setRuntimePath(currentRuntimePath()!)
                  }}
                >
                  <div class="i-hugeicons:arrow-turn-backward h-5 w-5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="ui:_trn_tooltip.undo" />
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger>
                <Button
                  rounded={false}
                  type="secondary"
                  class="h-10"
                  size="small"
                  onClick={() => {
                    setRuntimePath(initialRuntimePath()!)
                  }}
                >
                  <div class="i-hugeicons:cancel-01 h-5 w-5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="ui:_trn_tooltip.reset" />
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger>
                <Button
                  rounded={false}
                  type="primary"
                  class="h-10"
                  size="small"
                  backgroundColor={
                    currentValidation()?.classification === "potentially_valid"
                      ? "bg-yellow-900"
                      : undefined
                  }
                  disabled={!canApply()}
                  loading={isChangingRuntimePath()}
                  onClick={async () => {
                    if (!canApply()) return
                    const result = currentValidation()!
                    modalsContext?.openModal(
                      {
                        name: "ConfirmChangeRuntimePath"
                      },
                      {
                        runtimePath: result.path,
                        isTargetFolderAlreadyUsed:
                          result.classification === "potentially_valid"
                      }
                    )
                  }}
                >
                  <div class="i-hugeicons:arrow-reload-horizontal h-5 w-5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Switch>
                  <Match when={currentRuntimePath() === runtimePath()}>
                    <Trans key="ui:_trn_tooltip.rtp_unchanged" />
                  </Match>
                  <Match
                    when={
                      currentValidation()?.classification === "valid" ||
                      currentValidation()?.classification ===
                        "potentially_valid"
                    }
                  >
                    <Trans key="ui:_trn_tooltip.apply_and_restart" />
                  </Match>
                  <Match
                    when={
                      currentValidation()?.classification ===
                      "incomplete_migration"
                    }
                  >
                    <Trans key="ui:_trn_tooltip.rtp_incomplete_migration" />
                  </Match>
                  <Match
                    when={
                      currentValidation()?.classification ===
                      "other_incomplete_migration"
                    }
                  >
                    <Trans key="ui:_trn_tooltip.rtp_other_incomplete_migration" />
                  </Match>
                  <Match
                    when={
                      !currentValidation() ||
                      currentValidation()?.classification === "invalid"
                    }
                  >
                    <Trans key="ui:_trn_tooltip.rtp_not_valid" />
                  </Match>
                </Switch>
              </TooltipContent>
            </Tooltip>
          </Center>
        </Row>
      </RowsContainer>
    </>
  )
}

export default RuntimePath
