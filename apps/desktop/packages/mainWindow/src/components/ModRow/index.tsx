import { useGDNavigate } from "@/managers/NavigationManager";
import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { rspc, rspcFetch } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Button, createNotification, Popover, Spinner } from "@gd/ui";
import { formatDistanceToNowStrict } from "date-fns";
import {
  createSignal,
  getOwner,
  Match,
  mergeProps,
  onCleanup,
  onMount,
  runWithOwner,
  Show,
  Switch
} from "solid-js";
import OverviewPopover from "../OverviewPopover";
import {
  getDataCreation,
  getDownloads,
  getLogoUrl,
  getName,
  getProjectId,
  getSummary,
  isCurseForgeData,
  ModProps,
  ModRowProps
} from "@/utils/mods";
import Categories from "./Categories";
import Authors from "./Authors";
import ModDownloadButton from "../ModDownloadButton";

const ModRow = (props: ModRowProps) => {
  const owner = getOwner();
  const [loading, setLoading] = createSignal(false);
  const [isRowSmall, setIsRowSmall] = createSignal(false);

  const mergedProps = mergeProps({ type: "Modpack" }, props);
  const navigate = useGDNavigate();
  const addNotification = createNotification();

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        setLoading(false);
        addNotification("Instance successfully created.");
      },
      onError() {
        setLoading(false);
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        setLoading(false);
        navigate(`/library`);
      }
    }
  );

  const instanceId = () => (props as ModProps)?.instanceId;

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onMutate() {
        setLoading(true);
      },
      onSuccess(instanceId) {
        setLoading(true);
        prepareInstanceMutation.mutate(instanceId);
      },
      onError() {
        setLoading(false);
        addNotification("Error while downloading the modpack.", "error");
      }
    }
  );

  const handleExplore = () => {
    navigate(
      `/${mergedProps.type === "Modpack" ? "modpacks" : "mods"}/${getProjectId(
        props
      )}/${
        isCurseForgeData(props.data) ? "curseforge" : "modrinth"
      }?instanceId=${instanceId()}`
    );
  };

  const instanceCreationObj = (
    fileId?: number | string,
    projectId?: number | string
  ) => {
    return isCurseForgeData(props.data)
      ? {
          Curseforge: {
            file_id: (fileId as number) || props.data.curseforge.mainFileId,
            project_id: (projectId as number) || props.data.curseforge.id
          }
        }
      : {
          Modrinth: {
            project_id: projectId?.toString() || props.data.modrinth.project_id,
            version_id: fileId?.toString() as string
          }
        };
  };

  let containerRef: HTMLDivElement;
  let resizeObserver: ResizeObserver;

  onMount(() => {
    resizeObserver = new ResizeObserver((entries) => {
      // eslint-disable-next-line solid/reactivity
      window.requestAnimationFrame(() => {
        for (let entry of entries) {
          const cr = entry.contentRect;
          const shouldSetRowSmall = cr.width < 712;
          if (isRowSmall() !== shouldSetRowSmall) {
            setIsRowSmall(shouldSetRowSmall);
          }
        }
      });
    });

    resizeObserver.observe(containerRef);
  });

  onCleanup(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
  });

  const Title = () => {
    return (
      <div class="flex flex-col justify-between">
        <div class="flex justify-between w-full">
          <Popover
            noPadding
            noTip
            content={
              <OverviewPopover
                data={props}
                modrinthCategories={props.modrinthCategories?.filter(
                  (category) =>
                    category.project_type.includes(props.type.toLowerCase())
                )}
              />
            }
            placement="right-start"
            color="bg-darkSlate-900"
          >
            <h2
              class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 cursor-pointer hover:underline"
              onClick={() => handleExplore()}
              classList={{
                "max-w-140": !isRowSmall(),
                "max-w-90": isRowSmall()
              }}
            >
              {getName(props)}
            </h2>
          </Popover>
          <Categories
            modProps={props}
            isRowSmall={isRowSmall()}
            modrinthCategories={props.modrinthCategories?.filter((category) =>
              category.project_type.includes(props.type.toLowerCase())
            )}
          />
        </div>
        <div class="flex gap-4 items-center">
          <div class="flex items-center gap-2 text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:time-fill" />
            <div class="whitespace-nowrap text-sm">
              {formatDistanceToNowStrict(
                new Date(getDataCreation(props)).getTime()
              )}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:download-fill" />
            <div class="text-sm whitespace-nowrap">
              {formatDownloadCount(getDownloads(props))}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <i class="text-darkSlate-100 i-ri:user-fill" />
            <Authors modProps={props} isRowSmall={isRowSmall} />
          </div>
        </div>
      </div>
    );
  };

  return (
    <div
      ref={(el) => (containerRef = el)}
      class="flex flex-col gap-4 overflow-hidden relative p-5 bg-darkSlate-700 rounded-2xl box-border h-36"
    >
      <div class="absolute z-10 bg-gradient-to-r from-darkSlate-700 from-50% inset-0" />
      <div class="absolute inset-0 from-darkSlate-700 z-10 bg-gradient-to-t" />
      <Show when={getLogoUrl(props)}>
        <img
          class="absolute right-0 top-0 bottom-0 select-none w-1/2 z-0"
          src={getLogoUrl(props) as string}
        />
      </Show>
      <div class="flex w-full">
        <div class="flex gap-4 w-full">
          <div class="flex flex-col gap-2 w-full z-10 bg-repeat-none">
            <Title />
            <div class="flex justify-between w-full">
              <p class="text-sm overflow-hidden text-ellipsis m-0 text-darkSlate-50 max-w-full max-h-15">
                <Switch>
                  <Match when={isRowSmall()}>
                    {truncateText(getSummary(props), 60)}
                  </Match>
                  <Match when={!isRowSmall()}>
                    {truncateText(getSummary(props), 120)}
                  </Match>
                </Switch>
              </p>
              <div class="flex w-full justify-end items-end">
                <Switch>
                  <Match when={mergedProps.type === "Modpack"}>
                    <div class="flex items-center gap-3">
                      <Button
                        size={isRowSmall() ? "small" : "medium"}
                        type="outline"
                        onClick={() => handleExplore()}
                      >
                        <Trans key="instance.explore_modpack" />
                      </Button>
                      <Show when={loading()}>
                        <Button>
                          <Spinner />
                        </Button>
                      </Show>
                      <Show when={!loading()}>
                        <Button
                          size={isRowSmall() ? "small" : "medium"}
                          disabled={loading()}
                          onClick={async () => {
                            runWithOwner(owner, async () => {
                              if (props.type !== "Modpack") return;

                              const imgUrl = getLogoUrl(props);
                              if (imgUrl) loadIconMutation.mutate(imgUrl);

                              let fileVersion = undefined;
                              if (!isCurseForgeData(props.data)) {
                                const mrVersions = await rspcFetch(() => [
                                  "modplatforms.modrinth.getProjectVersions",
                                  {
                                    project_id: getProjectId(props)
                                  }
                                ]);

                                fileVersion = (mrVersions as any).data[0].id;
                              }

                              createInstanceMutation.mutate({
                                group: props.defaultGroup || 1,
                                use_loaded_icon: true,
                                notes: "",
                                name: getName(props),
                                version: {
                                  Modpack: instanceCreationObj(fileVersion)
                                }
                              });
                            });
                          }}
                        >
                          <Show when={loading()}>
                            <Spinner />
                          </Show>
                          <Show when={!loading()}>
                            <Trans key="instance.download_latest" />
                          </Show>
                        </Button>
                      </Show>
                    </div>
                  </Match>
                  <Match when={mergedProps.type === "Mod"}>
                    <div class="flex gap-3">
                      <Button
                        size={isRowSmall() ? "small" : "medium"}
                        type="outline"
                        onClick={() => handleExplore()}
                      >
                        <Trans key="instance.explore_modpack" />
                      </Button>
                      <ModDownloadButton
                        size={isRowSmall() ? "small" : "medium"}
                        projectId={getProjectId(props)}
                        isCurseforge={isCurseForgeData(props.data)}
                        instanceId={instanceId()}
                        instanceLocked={
                          (props as ModProps).instanceDetails?.modpack?.locked
                        }
                        instanceMods={(props as ModProps).instanceMods}
                      />
                    </div>
                  </Match>
                </Switch>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ModRow;
