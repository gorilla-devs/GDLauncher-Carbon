import { mcVersions } from "@/utils/mcVersion";
import { supportedModloaders } from "@/utils/sidebar";
import DefaultImg from "/assets/images/default-instance-img.png";
import { McType } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Checkbox, Dropdown } from "@gd/ui";
import { useSearchParams } from "@solidjs/router";
import { Match, Switch, createMemo, createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import { rspc } from "@/utils/rspcClient";
import { useInfiniteVersionsQuery } from "../InfiniteScrollVersionsQueryWrapper";
import { getInstanceImageUrl } from "@/utils/instances";

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
  type: "modpack" | "mod";
};

const ExploreVersionsNavbar = (props: Props) => {
  const [searchParams, _setSearchParams] = useSearchParams();
  const instanceId = () => parseInt(searchParams.instanceId, 10);

  const infiniteQuery = useInfiniteVersionsQuery();

  const [overrideEnabled, setOverrideEnabled] = createSignal(
    !instanceId || isNaN(instanceId())
  );

  const [gameVersionFilters, _setGameVersionFilters] = createStore({
    snapshot: false,
    oldAlpha: false,
    oldBeta: false
  });

  const instanceDetails = rspc.createQuery(() => [
    "instance.getInstanceDetails",
    instanceId()
  ]);

  const modloaders = () => {
    let res: { label: string; key: string }[] = [];

    if (props.modplatform === "modrinth") {
      const results = supportedModloaders[props.modplatform];
      res = results
        .filter((modloader) =>
          modloader.supported_project_types.includes("modpack")
        )
        .map((v) => ({
          label: v.name.toString(),
          key: v.name.toString()
        }));
    } else if (props.modplatform === "curseforge") {
      const results = supportedModloaders[props.modplatform];
      res = results.map((v) => ({
        label: v.toString(),
        key: v.toString()
      }));
    }

    return [
      {
        label: "Select a modloader",
        key: ""
      }
    ].concat(res);
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
    <div class="w-full flex gap-4 h-12 my-4">
      <Switch>
        <Match when={!isNaN(instanceId())}>
          <div class="flex gap-2">
            <div
              class="h-full flex-1 w-12"
              style={{
                "background-image": instanceDetails.data?.iconRevision
                  ? `url("${getInstanceImageUrl(
                      instanceId(),
                      instanceDetails.data?.iconRevision
                    )}")`
                  : `url("${DefaultImg}")`,
                "background-size": instanceDetails.data?.iconRevision
                  ? "cover"
                  : "120%"
              }}
            />
            <div class="flex flex-col justify-between">
              <div>{instanceDetails.data?.name}</div>
              <div class="flex text-darkSlate-50 gap-2">
                <Checkbox
                  checked={overrideEnabled()}
                  onChange={setOverrideEnabled}
                />
                <Trans key="rowcontainer.override_filters" />
              </div>
            </div>
          </div>
        </Match>
        <Match
          when={props.type === "mod" && (!instanceId || isNaN(instanceId()))}
        >
          <div class="flex items-center text-darkSlate-100">
            <Trans key="rowcontainer.no_instance_selected" />
          </div>
        </Match>
      </Switch>
      <div class="flex items-center gap-2">
        <Dropdown
          class="w-full"
          containerClass="w-full"
          options={filteredMappedGameVersions()}
          disabled={!overrideEnabled()}
          icon={<div class="i-ri:price-tag-3-fill" />}
          value={infiniteQuery.query.gameVersion || null}
          onChange={(val) => {
            infiniteQuery?.setQuery({
              gameVersion: val.key.toString() || null
            });
          }}
        />
      </div>
      <div class="flex items-center gap-2">
        <Dropdown
          class="w-full"
          containerClass="w-full"
          options={modloaders()!}
          disabled={!overrideEnabled()}
          icon={<div class="i-ri:price-tag-3-fill" />}
          value={infiniteQuery.query.modLoaderType || null}
          onChange={(val) => {
            infiniteQuery?.setQuery({
              modLoaderType: val.key.toString() || null
            });
          }}
        />
      </div>
    </div>
  );
};

export default ExploreVersionsNavbar;
