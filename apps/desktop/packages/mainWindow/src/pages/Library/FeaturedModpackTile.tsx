import { useGDNavigate } from "@/managers/NavigationManager";
import { rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Skeleton } from "@gd/ui";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createResource,
  createSignal
} from "solid-js";

const HEXING_TALES_MODPACK_ID = 891604;

const FeaturedModpackTile = () => {
  const navigate = useGDNavigate();
  const rspcContext = rspc.useContext();
  const [shouldShow, setShouldShow] = createSignal(true);

  const [hexingTales] = createResource(() => {
    return rspcContext.client.query([
      "modplatforms.curseforge.getMod",
      {
        modId: HEXING_TALES_MODPACK_ID
      }
    ]);
  });

  const instances = rspc.createQuery(() => ({
    queryKey: ["instance.getAllInstances"]
  }));

  createEffect(() => {
    if (!instances.data) return;

    for (const i of instances.data) {
      if (
        i.status.status === "valid" &&
        i.status.value.modpack?.type === "curseforge" &&
        i.status.value.modpack?.value?.project_id === HEXING_TALES_MODPACK_ID
      ) {
        setShouldShow(false);
        return;
      }
    }

    setShouldShow(true);
  });

  return (
    <Show when={shouldShow()}>
      <>
        <div class="h-24 w-[1px] bg-darkSlate-400" />
        <div
          class="w-70 h-24 relative hover:bg-darkSlate-700 duration-200 ease-in-out box-border rounded-md group overflow-hidden hover:outline outline-2 outline-darkSlate-500"
          onClick={() => {
            navigate(`/modpacks/${HEXING_TALES_MODPACK_ID}/curseforge`);
          }}
        >
          <div class="absolute top-0 left-0 group-hover:-translate-y-full duration-200 ease-in-out">
            <Trans key="featured.try_featured_modpack" />
          </div>
          <Switch>
            <Match when={hexingTales()?.data}>
              <div class="w-full h-full relative">
                <img
                  src={hexingTales()?.data.logo?.url}
                  class="absolute left-0 bottom-0 w-16 h-16 rounded-lg duration-200 ease-in-out group-hover:scale-130 group-hover:translate-x-4 group-hover:-translate-y-4"
                />
                <div class="absolute left-20 bottom-0 group-hover:opacity-0 duration-200 ease-in-out">
                  <div class="text-xl font-bold text-nowrap">
                    {hexingTales()?.data.name}
                  </div>
                  <div class="text-sm text-darkSlate-50">
                    <For each={hexingTales()?.data.authors}>
                      {(v) => <span>{v.name}</span>}
                    </For>
                  </div>
                </div>

                <div class="absolute left-40 top-1/2 -translate-y-1/2 translate-x-[150%] group-hover:translate-x-0 duration-200 ease-in-out">
                  <Trans key="featured.show_more" />
                </div>
              </div>
            </Match>
            <Match when={!hexingTales()?.data}>
              <div class="relative w-full h-full">
                <div class="absolute bottom-0 left-0 w-full">
                  <Skeleton.featuredHomeTile />
                </div>
              </div>
            </Match>
          </Switch>
        </div>
      </>
    </Show>
  );
};

export default FeaturedModpackTile;
