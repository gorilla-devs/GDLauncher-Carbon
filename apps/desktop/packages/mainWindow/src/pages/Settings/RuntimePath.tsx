import { Button, Input, Tooltip } from "@gd/ui";
import { Trans } from "@gd/i18n";
import PageTitle from "./components/PageTitle";
import RowsContainer from "./components/RowsContainer";
import Row from "./components/Row";
import Title from "./components/Title";
import {
  createEffect,
  createResource,
  createSignal,
  Match,
  Switch
} from "solid-js";
import Center from "./components/Center";
import { useModal } from "@/managers/ModalsManager";

const [isChangingRuntimePath, setIsChangingRuntimePath] = createSignal(false);

const RuntimePath = () => {
  const modalsContext = useModal();

  const [runtimePath, setRuntimePath] = createSignal<string | undefined>(
    undefined
  );

  const [initialRuntimePath] = createResource(() => {
    return window.getInitialRuntimePath();
  });

  const [currentRuntimePath] = createResource(() => {
    return window.getRuntimePath();
  });

  createEffect(() => {
    if (currentRuntimePath() === undefined) {
      return;
    }

    setRuntimePath(currentRuntimePath()!);
  });

  const [isPathValid] = createResource(
    () => [runtimePath()] as const,
    () => {
      return window.validateRuntimePath(runtimePath()!);
    }
  );

  return (
    <>
      <PageTitle>
        <Trans key="settings:RuntimePath" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title description={<Trans key="settings:runtime_path_text" />}>
            <Trans key="settings:runtime_path_title" />
          </Title>
          <Center>
            <Input
              class="w-full"
              value={runtimePath()}
              icon={
                <div
                  class="w-5 h-5 cursor-pointer hover:text-darkSlate-100 ease-in-out transition-colors i-ri:folder-fill"
                  onClick={async () => {
                    const result = await window.openFileDialog({
                      title: "Select Runtime Path",
                      defaultPath: runtimePath(),
                      properties: ["openDirectory"]
                    });

                    if (result.canceled) {
                      return;
                    }

                    setRuntimePath(result.filePaths[0]);
                  }}
                />
              }
              onInput={({ target: { value } }) => {
                setRuntimePath(value);
              }}
            />
            <Tooltip content={<Trans key="tooltip.undo" />}>
              <Button
                rounded={false}
                type="secondary"
                class="h-10"
                size="small"
                onClick={() => {
                  setRuntimePath(currentRuntimePath()!);
                }}
              >
                <i class="w-5 h-5 i-ri:arrow-go-back-fill" />
              </Button>
            </Tooltip>
            <Tooltip content={<Trans key="tooltip.reset" />}>
              <Button
                rounded={false}
                type="secondary"
                class="h-10"
                size="small"
                onClick={() => {
                  setRuntimePath(initialRuntimePath()!);
                }}
              >
                <i class="w-5 h-5 i-ri:close-fill" />
              </Button>
            </Tooltip>
            <Tooltip
              content={
                <Switch>
                  <Match when={isPathValid()}>
                    <Trans key="tooltip.apply_and_restart" />
                  </Match>
                  <Match when={!isPathValid()}>
                    <Trans key="tooltip.rtp_not_valid" />
                  </Match>
                </Switch>
              }
            >
              <Button
                rounded={false}
                type="primary"
                class="h-10"
                size="small"
                disabled={!isPathValid() || isChangingRuntimePath()}
                loading={isChangingRuntimePath()}
                onClick={async () => {
                  modalsContext?.openModal(
                    {
                      name: "ConfirmChangeRuntimePath"
                    },
                    {
                      runtimePath: runtimePath()!,
                      setIsChangingRuntimePath,
                      isChangingRuntimePath: isChangingRuntimePath
                    }
                  );
                }}
              >
                <i class="w-5 h-5 i-ri-restart-line" />
              </Button>
            </Tooltip>
          </Center>
        </Row>
      </RowsContainer>
    </>
  );
};

export default RuntimePath;
