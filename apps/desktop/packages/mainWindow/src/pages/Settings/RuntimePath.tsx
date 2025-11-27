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
  Switch
} from "solid-js"
import Center from "./components/Center"
import { useModal } from "@/managers/ModalsManager"

const [isChangingRuntimePath, setIsChangingRuntimePath] = createSignal(false)

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

  const [isPathValid] = createResource(
    () => [runtimePath()] as const,
    () => {
      return window.validateRuntimePath(runtimePath()!)
    }
  )

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
                  class="i-hugeicons:folder-01 hover:text-lightSlate-700 h-5 w-5 cursor-pointer transition-colors ease-in-out"
                  onClick={async () => {
                    const result = await window.openFileDialog({
                      title: "Select Runtime Path",
                      defaultPath: runtimePath(),
                      properties: ["openDirectory"]
                    })

                    if (result.canceled) {
                      return
                    }

                    setRuntimePath(result.filePaths[0])
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
                    isPathValid() === "potentially_valid"
                      ? "bg-yellow-900"
                      : undefined
                  }
                  disabled={
                    !isPathValid() ||
                    isPathValid() === "invalid" ||
                    isChangingRuntimePath() ||
                    currentRuntimePath() === runtimePath()
                  }
                  loading={isChangingRuntimePath()}
                  onClick={async () => {
                    modalsContext?.openModal(
                      {
                        name: "ConfirmChangeRuntimePath"
                      },
                      {
                        runtimePath: runtimePath()!,
                        isTargetFolderAlreadyUsed:
                          isPathValid() === "potentially_valid",
                        setIsChangingRuntimePath,
                        isChangingRuntimePath: isChangingRuntimePath
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
                      isPathValid() === "valid" ||
                      isPathValid() === "potentially_valid"
                    }
                  >
                    <Trans key="ui:_trn_tooltip.apply_and_restart" />
                  </Match>
                  <Match when={!isPathValid() || isPathValid() === "invalid"}>
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
