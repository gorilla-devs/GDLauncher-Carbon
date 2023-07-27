/* eslint-disable solid/no-innerhtml */
import { Button, Spinner } from "@gd/ui";
import { ModalProps, useModal } from "../..";
import ModalLayout from "../../ModalLayout";
import { FEUnifiedSearchResult } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Match, Show, Switch, createEffect, createSignal } from "solid-js";
import { format } from "date-fns";
import { rspc } from "@/utils/rspcClient";
import { lastInstanceOpened } from "@/utils/routes";
import {
  getDataCreation,
  getFileId,
  getLatestVersion,
  getLogoUrl,
  getName,
  isCurseForgeData,
} from "@/utils/Mods";
import Authors from "@/components/ModRow/Authors";

const ModDetails = (props: ModalProps) => {
  const [loading, setLoading] = createSignal(false);
  const modDetails = () => props.data?.mod as FEUnifiedSearchResult;
  const modId = () => getFileId(modDetails());
  const modalsContext = useModal();
  const [modpackDescription, setModpackDescription] = createSignal("");

  const installModMutation = rspc.createMutation(["instance.installMod"], {
    onMutate() {
      setLoading(true);
    },
    onSettled() {
      setLoading(false);
    },
  });

  createEffect(() => {
    if (modId() && isCurseForgeData(modDetails())) {
      const modpackDescription = rspc.createQuery(() => [
        "modplatforms.curseforge.getModDescription",
        { modId: modId() as number },
      ]);
      if (modpackDescription.data?.data)
        setModpackDescription(modpackDescription.data?.data);
    }
  });

  // createEffect(() => {
  //   console.log("modDetails", modDetails());
  //   if (modId() && !isCurseForgeData(modDetails())) {
  //     const modpackDescription = rspc.createQuery(() => [
  //       "modplatforms.modrinth.getVersion",
  //       modId() as string,
  //     ]);
  //     console.log("TEST", modId(), modpackDescription.data?.changelog);
  //     if (modpackDescription.data?.changelog)
  //       setModpackDescription(modpackDescription.data.changelog);
  //   }
  // });

  let refStickyTabs: HTMLDivElement;
  const [isSticky, setIsSticky] = createSignal(false);

  const instanceCreationObj = (
    fileId?: number | string,
    projectId?: number | string
  ) => {
    return isCurseForgeData(props.data)
      ? {
          Curseforge: {
            file_id: (fileId as number) || props.data.curseforge.mainFileId,
            project_id: (projectId as number) || props.data.curseforge.id,
          },
        }
      : {
          Modrinth: {
            project_id: projectId?.toString() || props.data.modrinth.project_id,
            version_id: fileId?.toString() as string,
          },
        };
  };

  return (
    <ModalLayout noHeader={props.noHeader} title={props?.title}>
      <div class="h-130 w-190">
        <Switch>
          <Match when={props.data}>
            <div
              class="relative h-full bg-darkSlate-800 overflow-auto max-h-full overflow-x-hidden"
              style={{
                "scrollbar-gutter": "stable",
              }}
              onScroll={() => {
                const rect = refStickyTabs.getBoundingClientRect();
                setIsSticky(rect.top <= 80);
              }}
            >
              <div class="flex flex-col justify-between ease-in-out transition-all h-52 items-stretch">
                <div class="relative h-full">
                  <div
                    class="h-full absolute left-0 right-0 top-0 bg-cover bg-center bg-fixed bg-no-repeat"
                    style={{
                      "background-image": `url("${getLogoUrl(modDetails())}")`,
                      "background-position": "right-5rem",
                    }}
                  />
                  <div class="z-10 sticky top-5 left-5 w-fit">
                    <Button
                      onClick={() => {
                        modalsContext?.closeModal();
                      }}
                      icon={<div class="text-2xl i-ri:arrow-drop-left-line" />}
                      size="small"
                      type="transparent"
                    >
                      <Trans
                        key="instance.step_back"
                        options={{
                          defaultValue: "Back",
                        }}
                      />
                    </Button>
                  </div>
                  <div class="flex justify-center sticky px-4 z-20 bg-gradient-to-t h-24 top-52 from-darkSlate-800 from-10%">
                    <div class="flex gap-4 w-full lg:flex-row">
                      <div
                        class="bg-darkSlate-800 h-16 w-16 rounded-xl bg-center bg-cover"
                        style={{
                          "background-image": `url("${getLogoUrl(
                            modDetails()
                          )}")`,
                        }}
                      />
                      <div class="flex flex-1 flex-col max-w-185">
                        <div class="flex gap-4 items-center cursor-pointer">
                          <h1 class="m-0 h-9">{getName(modDetails())}</h1>
                        </div>
                        <div class="flex flex-col lg:flex-row justify-between cursor-default">
                          <div class="flex flex-col lg:flex-row text-darkSlate-50 gap-1 items-start lg:items-center lg:gap-0">
                            <div class="p-0 lg:pr-4 border-0 lg:border-r-2 border-darkSlate-500">
                              {getLatestVersion(modDetails())}
                            </div>
                            <Show when={getDataCreation(modDetails())}>
                              <div class="p-0 border-0 lg:border-r-2 border-darkSlate-500 flex gap-2 items-center lg:px-4">
                                <div class="i-ri:time-fill" />

                                {format(
                                  new Date(
                                    getDataCreation(modDetails())
                                  ).getTime(),
                                  "P"
                                )}
                              </div>
                            </Show>
                            <div class="p-0 lg:px-4 flex gap-2 items-center">
                              <div class="i-ri:user-fill" />
                              <div class="text-sm flex gap-2 whitespace-nowrap overflow-x-auto max-w-52">
                                <Authors modProps={modDetails()} />
                              </div>
                            </div>
                          </div>
                          <div class="flex items-center gap-2 mt-2 lg:mt-0">
                            <Button
                              disabled={loading()}
                              uppercase
                              type="glow"
                              size="large"
                              onClick={() => {
                                installModMutation.mutate({
                                  mod_source: instanceCreationObj(),
                                  instance_id: parseInt(
                                    lastInstanceOpened(),
                                    10
                                  ),
                                });
                              }}
                            >
                              <Show when={loading()}>
                                <Spinner />
                              </Show>
                              <Show when={!loading()}>
                                <Trans
                                  key="mod.download"
                                  options={{
                                    defaultValue: "Download",
                                  }}
                                />
                              </Show>
                            </Button>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <Show when={isSticky()}>
                <div class="bg-darkSlate-900 w-full sticky top-0 flex justify-between items-center px-4 py-2 box-border">
                  <Show when={isSticky()}>
                    <Button
                      onClick={() => {
                        modalsContext?.closeModal();
                      }}
                      icon={<div class="text-2xl i-ri:arrow-drop-left-line" />}
                      size="small"
                      type="secondary"
                    >
                      <Trans
                        key="instance.step_back"
                        options={{
                          defaultValue: "Back",
                        }}
                      />
                    </Button>
                  </Show>
                  <Show when={isSticky()}>
                    <Button
                      disabled={loading()}
                      uppercase
                      type="glow"
                      size="small"
                      onClick={() => {
                        installModMutation.mutate({
                          mod_source: instanceCreationObj(),
                          instance_id: parseInt(lastInstanceOpened(), 10),
                        });
                      }}
                    >
                      <Show when={loading()}>
                        <Spinner />
                      </Show>
                      <Show when={!loading()}>
                        <Trans
                          key="mod.download"
                          options={{
                            defaultValue: "Download",
                          }}
                        />
                      </Show>
                    </Button>
                  </Show>
                </div>
              </Show>
              <div
                class="p-4"
                ref={(el) => {
                  refStickyTabs = el;
                }}
                innerHTML={modpackDescription()}
              />
            </div>
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  );
};

export default ModDetails;
