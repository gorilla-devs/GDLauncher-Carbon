import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";
import { Modpack } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Skeleton } from "@gd/ui";
import {
  For,
  Match,
  Switch,
  createEffect,
  createResource,
  createSignal,
  getOwner,
  runWithOwner
} from "solid-js";

const HEXING_TALES_MODPACK_ID = 891604;

const FeaturedModpackTile = () => {
  const owner = getOwner();
  const navigate = useGDNavigate();
  const rspcContext = rspc.useContext();
  const [loading, setLoading] = createSignal(false);
  const [taskId, setTaskId] = createSignal<number | null>(null);

  const trackDownload = rspc.createMutation(["metrics.sendEvent"]);

  const [hexingTales] = createResource(() => {
    return rspcContext.client.query([
      "modplatforms.curseforge.getMod",
      {
        modId: HEXING_TALES_MODPACK_ID
      }
    ]);
  });

  const prepareInstanceMutation = rspc.createMutation([
    "instance.prepareInstance"
  ]);

  const loadIconMutation = rspc.createMutation(["instance.loadIconUrl"]);

  const createInstanceMutation = rspc.createMutation([
    "instance.createInstance"
  ]);

  const task = rspc.createQuery(() => ["vtask.getTask", taskId()]);

  createEffect((prev) => {
    if (taskId() && prev && !task.data) {
      setLoading(false);
      setTaskId(null);
    }

    return task.data;
  });

  return (
    <div class="w-70 h-auto">
      <h3>
        <Trans key="featured.try_featured_modpack">
          {""}
          <span class="text-primary-400" />
          {""}
        </Trans>
      </h3>
      <Switch>
        <Match when={hexingTales()?.data}>
          <div class="flex gap-4 items-center">
            <img
              src={hexingTales()?.data.logo?.url}
              class="w-16 h-16 rounded-lg"
            />
            <div>
              <div class="text-xl font-bold">{hexingTales()?.data.name}</div>
              <div class="text-sm text-darkSlate-50">
                <For each={hexingTales()?.data.authors}>
                  {(v) => <span>{v.name}</span>}
                </For>
              </div>
            </div>
          </div>
          <p>{hexingTales()?.data.summary}</p>
          <div class="flex justify-between w-full gap-4">
            <Button
              loading={loading()}
              onClick={async () => {
                runWithOwner(owner, async () => {
                  setLoading(true);
                  trackDownload.mutate({
                    data: {
                      campaign_id: "featured_gdl_content",
                      item_id: HEXING_TALES_MODPACK_ID.toString()
                    },
                    event_name: "featured_modpack_installed"
                  });

                  const creationObject = {
                    Curseforge: {
                      file_id: hexingTales()?.data.latestFiles[0].id,
                      project_id: hexingTales()?.data.id
                    }
                  };

                  const defaultGroup = await rspcContext.client.query([
                    "instance.getDefaultGroup"
                  ]);

                  const imgUrl = hexingTales()?.data.logo?.thumbnailUrl;
                  if (imgUrl) await loadIconMutation.mutateAsync(imgUrl);

                  const instanceId = await createInstanceMutation.mutateAsync({
                    group: defaultGroup,
                    use_loaded_icon: true,
                    notes: "",
                    name: hexingTales()?.data.name!,
                    version: {
                      Modpack: creationObject as Modpack
                    }
                  });

                  const taskId =
                    await prepareInstanceMutation.mutateAsync(instanceId);

                  setTaskId(taskId);
                });
              }}
            >
              <Trans key="featured.download" />
            </Button>
            <Button
              type="secondary"
              onClick={() => {
                navigate(`/modpacks/${HEXING_TALES_MODPACK_ID}/curseforge`);
              }}
            >
              <Trans key="featured.show_more" />
            </Button>
          </div>
        </Match>
        <Match when={!hexingTales()?.data}>
          <Skeleton.featuredHomeTile />
        </Match>
      </Switch>
    </div>
  );
};

export default FeaturedModpackTile;
