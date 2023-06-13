import { FEFile, FEMod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { format } from "date-fns";
import { rspc } from "@/utils/rspcClient";
import { useGDNavigate } from "@/managers/NavigationManager";
import { Spinner, createNotification } from "@gd/ui";
import { Match, Switch, createSignal } from "solid-js";

type Props = {
  modVersion: FEFile;
  project: FEMod;
};

const VersionRow = (props: Props) => {
  const [loading, setLoading] = createSignal(false);
  const navigate = useGDNavigate();
  const addNotification = createNotification();

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const prepareInstanceMutation = rspc.createMutation(
    ["instance.prepareInstance"],
    {
      onSuccess() {
        addNotification("Instance successfully created.");
      },
      onError() {
        setLoading(false);
        addNotification("Error while creating the instance.", "error");
      },
      onSettled() {
        navigate(`/library`);
      },
    }
  );

  const createInstanceMutation = rspc.createMutation(
    ["instance.createInstance"],
    {
      onSuccess(instanceId) {
        setLoading(true);
        prepareInstanceMutation.mutate(instanceId);
      },
      onError() {
        setLoading(false);
        addNotification("Error while downloading the modpack.", "error");
      },
    }
  );

  return (
    <div class="flex flex-col py-2">
      <h4 class="font-medium m-0">{props.modVersion.displayName}</h4>
      <div class="flex justify-between">
        <div class="flex justify-between">
          <div class="flex justify-between text-sm divide-darkSlate-500 text-lightGray-800 divide-x-1">
            <span class="pr-3">{props.modVersion.gameVersions[0]}</span>
            <span class="px-3">
              {format(new Date(props.modVersion.fileDate), "dd-MM-yyyy")}
            </span>
            <span
              class="pl-3"
              classList={{
                "text-green-500": props.modVersion.releaseType === "stable",
                "text-yellow-500": props.modVersion.releaseType === "beta",
                "text-red-500": props.modVersion.releaseType === "alpha",
              }}
            >
              {props.modVersion.releaseType}
            </span>
          </div>
        </div>
        <span
          class="flex gap-2 text-lightGray-800 cursor-pointer select-none"
          onClick={() => {
            loadIconMutation.mutate(props.project.logo.url);
            createInstanceMutation.mutate({
              group: defaultGroup.data || 1,
              use_loaded_icon: true,
              notes: "",
              name: props.modVersion.displayName,
              version: {
                Modpack: {
                  Curseforge: {
                    file_id: props.modVersion.id,
                    project_id: props.modVersion.modId,
                  },
                },
              },
            });
          }}
        >
          <Switch>
            <Match when={loading()}>
              <Trans
                key="modpack.version_downloading"
                options={{
                  defaultValue: "Downloading...",
                }}
              />
              <Spinner class="w-5 h-5" />
            </Match>
            <Match when={!loading()}>
              <Trans
                key="modpack.version_download"
                options={{
                  defaultValue: "Download version",
                }}
              />
              <div class="i-ri:download-2-line" />
            </Match>
          </Switch>
        </span>
      </div>
    </div>
  );
};

export default VersionRow;
