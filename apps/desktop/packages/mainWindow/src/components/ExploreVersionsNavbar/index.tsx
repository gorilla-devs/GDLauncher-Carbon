import { fetchImage } from "@/utils/instances";
import { mcVersions } from "@/utils/mcVersion";
import { supportedModloaders } from "@/utils/sidebar";
import DefaultImg from "/assets/images/default-instance-img.png";
import { McType } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Checkbox, Dropdown } from "@gd/ui";
import { useSearchParams } from "@solidjs/router";
import { Match, Switch, createMemo, createResource } from "solid-js";
import { createStore } from "solid-js/store";
import { rspc } from "@/utils/rspcClient";

const mapTypeToColor = (type: McType) => {
  return (
    <Switch>
      <Match when={type === "release"}>
        <span class="text-green-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "snapshot"}>
        <span class="text-yellow-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_alpha"}>
        <span class="text-purple-500">{`[${type}]`}</span>
      </Match>
      <Match when={type === "old_beta"}>
        <span class="text-red-500">{`[${type}]`}</span>
      </Match>
    </Switch>
  );
};

type Props = {
  modplatform: "curseforge" | "modrinth";
};

const ExploreVersionsNavbar = (props: Props) => {
  const [searchParams, _setSearchParams] = useSearchParams();
  const [gameVersionFilters, setGameVersionFilters] = createStore({
    snapshot: false,
    oldAlpha: false,
    oldBeta: false
  });

  const instanceId = parseInt(searchParams.instanceId, 10);

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    instanceId
  ]);

  const [imageResource, { refetch }] = createResource(
    () => instanceId,
    fetchImage
  );

  const modloaders = () => {
    if (props.modplatform === "modrinth") {
      const results = supportedModloaders[props.modplatform];
      return results
        .filter((modloader) =>
          // TODO: FIX
          modloader.supported_project_types.includes("modpack")
        )
        .map((v) => ({
          label: v.name.toString(),
          key: v.name.toString()
        }));
    } else if (props.modplatform === "curseforge") {
      const results = supportedModloaders[props.modplatform];
      return results.map((v) => ({
        label: v.toString(),
        key: v.toString()
      }));
    }
  };

  const filteredGameVersions = createMemo(() => {
    const snapshot = gameVersionFilters.snapshot;
    const oldAlpha = gameVersionFilters.oldAlpha;
    const oldBeta = gameVersionFilters.oldBeta;

    return mcVersions().filter(
      (item) =>
        item.type === "release" ||
        (item.type === "snapshot" && snapshot) ||
        (item.type === "old_beta" && oldBeta) ||
        (item.type === "old_alpha" && oldAlpha)
    );
  });

  const filteredMappedGameVersions = () => {
    const allVersionsLabel = {
      label: (
        <span>
          <Trans key="minecraft_all_versions" />
        </span>
      ),
      key: ""
    };

    return [
      allVersionsLabel,
      ...filteredGameVersions().map((item) => ({
        label: (
          <div class="flex justify-between w-full">
            <span>{item.id}</span>
            {mapTypeToColor(item.type)}
          </div>
        ),
        key: item.id
      }))
    ];
  };

  return (
    <div class="h-12 w-full flex gap-4 my-4">
      <div class="flex gap-2">
        <div
          class="h-full w-12 flex-1"
          style={{
            "background-image": imageResource()
              ? `url("${imageResource() as string}")`
              : `url("${DefaultImg}")`,
            "background-size": imageResource() ? "cover" : "120%"
          }}
        />
        <div class="flex flex-col justify-between">
          <div>{instanceDetails.data?.name}</div>
          <div class="flex text-darkSlate-50 gap-2">
            <Checkbox />
            Override filters
          </div>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <Dropdown
          class="w-full"
          containerClass="w-full"
          options={filteredMappedGameVersions()}
          // disabled={!isNaN(instanceId()!)}
          icon={<div class="i-ri:price-tag-3-fill" />}
          // value={infiniteQuery.query.gameVersions?.[0] || null}
          // onChange={(val) => {
          //   infiniteQuery?.setQuery({
          //     gameVersions: val.key ? [val.key as string] : null
          //   });
          // }}
        />
      </div>
      <div class="flex items-center gap-2">
        <Dropdown
          class="w-full"
          containerClass="w-full"
          options={modloaders()!}
          // disabled={!isNaN(instanceId()!)}
          icon={<div class="i-ri:price-tag-3-fill" />}
          // value={infiniteQuery.query.gameVersions?.[0] || null}
          // onChange={(val) => {
          //   infiniteQuery?.setQuery({
          //     gameVersions: val.key ? [val.key as string] : null
          //   });
          // }}
        />
      </div>
    </div>
  );
};

export default ExploreVersionsNavbar;
