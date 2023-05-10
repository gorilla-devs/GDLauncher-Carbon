import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input, Spinner } from "@gd/ui";
import { For, Match, Show, Switch, createEffect, createSignal } from "solid-js";
import glassBlock from "/assets/images/icons/glassBlock.png";
import Modpack from "./Modpack";
import LogoDark from "/assets/images/logo-dark.svg";
import { useModal } from "@/managers/ModalsManager";
import { rspc } from "@/utils/rspcClient";
import { createStore } from "solid-js/store";
import {
  FEMod,
  FEModLoaderType,
  FEModSearchSortField,
} from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { RSPCError } from "@rspc/client";
import { mcVersions } from "@/utils/mcVersion";
import { deepTrack } from "@solid-primitives/deep";
import { modLoader, selectedModpackCategory } from "@/utils/modpackBrowser";
import useModpacksQuery from "./useModpacksQuery";

const NoModpacks = () => {
  return (
    <div class="w-full flex h-full items-center justify-center min-h-90">
      <div class="flex justify-center items-center flex-col text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.no_modpacks_text"
            options={{
              defaultValue: "At the moment there is no modpacks.",
            }}
          />
        </p>
      </div>
    </div>
  );
};

const NoMoreModpacks = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-56">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.fetching_no_more_modpacks"
            options={{
              defaultValue: "No more modpacks to load",
            }}
          />
        </p>
      </div>
    </div>
  );
};

const FetchingModpacks = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-56">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.fetching_modpacks_text"
            options={{
              defaultValue: "Loading modpacks",
            }}
          />
        </p>
        <Spinner />
      </div>
    </div>
  );
};

const ErrorFetchingModpacks = (props: { error: RSPCError | null }) => {
  const parsedError = () =>
    props.error?.message && JSON.parse(props.error?.message);
  return (
    <div class="w-full flex h-full justify-center items-center min-h-90">
      <div class="flex justify-center items-center flex-col text-center">
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.fetching_modpacks_error"
            options={{
              defaultValue: "There was an error while fetching modpacks",
            }}
          />
          {parsedError().cause[0].display}
        </p>
      </div>
    </div>
  );
};

const sortFields: Array<FEModSearchSortField> = [
  "featured",
  "popularity",
  "lastUpdated",
  "name",
  "author",
  "totalDownloads",
  "category",
  "gameVersion",
];

export default function Browser() {
  const modalsContext = useModal();
  const [t] = useTransContext();
  const [modpacks, setModpacks] = createStore<FEMod[]>([]);
  const [mappedMcVersions, setMappedMcVersions] = createSignal<
    { label: string; key: string }[]
  >([]);
  const [query, setQuery, incrementIndex, replaceList] = useModpacksQuery({
    categoryId: null,
    classId: "modpacks",
    gameId: 432,
    gameVersion: mappedMcVersions()[0]?.key || "",
    page: 1,
    modLoaderType: "any",
    sortField: "featured",
    sortOrder: "descending",
    pageSize: 40,
    slug: "",
    searchFilter: "",
    gameVersionTypeId: null,
    authorId: null,
    index: 0,
  });

  const curseforgeSearch = rspc.createQuery(() => [
    "modplatforms.curseforgeSearch",
    query,
  ]);

  let containerRef: HTMLDivElement;

  const rowVirtualizer = createVirtualizer({
    get count() {
      return modpacks.length + 1;
    },
    getScrollElement: () => containerRef,
    estimateSize: () => 240,
    overscan: 40,
  });

  createEffect(() => {
    if (
      modLoader() === undefined ||
      modLoader() === null ||
      modLoader() === query.query.modLoaderType
    )
      return;
    rowVirtualizer.scrollToIndex(0);
    setQuery({ modLoaderType: modLoader() as FEModLoaderType });
  });

  createEffect(() => {
    if (selectedModpackCategory() === query.query.categoryId) return;
    rowVirtualizer.scrollToIndex(0);
    const isAll = selectedModpackCategory() === "all";
    setQuery({
      categoryId: isAll ? null : (selectedModpackCategory() as number),
    });
  });

  createEffect(() => {
    mcVersions().forEach((version) => {
      if (version.type === "release") {
        setMappedMcVersions((prev) => [
          ...prev,
          { label: version.id, key: version.id },
        ]);
      }
    });
  });

  const index = () =>
    (curseforgeSearch.data?.pagination &&
      curseforgeSearch.data.pagination.index) ||
    0;

  const totalCount = () =>
    (curseforgeSearch.data?.pagination &&
      curseforgeSearch.data.pagination.totalCount) ||
    1;

  const hasNextPage = () => index() < totalCount();

  createEffect(() => {
    const pageSize = query.query.pageSize || 40;
    const [lastItem] = [...rowVirtualizer.getVirtualItems()].reverse();
    // const reload =  pageSize &&
    console.log(
      "BEFORE_NEXT_PAGE",
      modpacks.length,
      query.query.index,
      index() + pageSize,
      modpacks,
      lastItem.index
    );
    if (
      !lastItem ||
      modpacks.length === 0 ||
      modpacks.length === index() + pageSize
    )
      return;
    if (
      lastItem.index >= modpacks.length - 1 &&
      !curseforgeSearch.isLoading &&
      !curseforgeSearch.isInitialLoading &&
      hasNextPage()
    ) {
      console.log(
        "NEXT_PAGE"
        // firstIncrement,
        // modpacks.length
        // query.query.index,
        // lastItem.index
      );
      incrementIndex();
    }
  });

  createEffect(() => {
    // console.log("QUERY-OBJ", deepTrack(query), query.query.index);
    console.log("QUERY-OBJ", deepTrack(query), query.query.index);
  });

  createEffect(() => {
    console.log("replaceList", replaceList());
  });

  createEffect(() => {
    console.log("curseforgeSearch", curseforgeSearch.data?.data);
  });

  createEffect(() => {
    if (curseforgeSearch.data?.data && !curseforgeSearch.isFetching) {
      if (!replaceList()) {
        console.log("ADD");
        setModpacks((prev) => [...prev, ...curseforgeSearch.data.data]);
      }
    }
  });
  createEffect(() => {
    if (curseforgeSearch.data?.data && !curseforgeSearch.isFetching) {
      if (replaceList()) {
        console.log("UPDATE");
        setModpacks(curseforgeSearch.data.data);
      }
    }
  });

  return (
    <div class="box-border h-full w-full relative">
      <div class="flex flex-col bg-darkSlate-800 pt-5 sticky top-0 left-0 right-0 z-10 px-5">
        <div class="flex items-center justify-between gap-3 pb-4 flex-wrap">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full text-darkSlate-50 rounded-full flex-1 max-w-none"
            onInput={(e) => {
              const target = e.target as HTMLInputElement;
              rowVirtualizer.scrollToIndex(0);
              setQuery({ searchFilter: target.value });
            }}
          />
          <div class="flex items-center gap-3">
            <p class="text-darkSlate-50">
              <Trans
                key="instance.sort_by"
                options={{
                  defaultValue: "Sort by:",
                }}
              />
            </p>
            <Dropdown
              options={sortFields.map((field) => ({
                label: t(`instance.sort_by_${field}`),
                key: field,
              }))}
              onChange={(val) => {
                rowVirtualizer.scrollToIndex(0);
                setQuery({ sortField: val.key as FEModSearchSortField });
              }}
              value={0}
              rounded
            />
            <Dropdown
              options={mappedMcVersions()}
              icon={<div class="i-ri:price-tag-3-fill" />}
              rounded
              bgColorClass="bg-darkSlate-400"
              value={mappedMcVersions()[0].key}
              onChange={(val) => {
                rowVirtualizer.scrollToIndex(0);
                setQuery({ gameVersion: val.key as string });
              }}
            />
          </div>
          <div
            class="cursor-pointer text-2xl"
            classList={{
              "i-ri:sort-asc": query.query.sortOrder === "ascending",
              "i-ri:sort-desc": query.query.sortOrder === "descending",
            }}
            onClick={() => {
              const isAsc = query.query.sortOrder === "ascending";
              setQuery({ sortOrder: isAsc ? "descending" : "ascending" });
            }}
          />
          <Button
            variant="outline"
            size="medium"
            icon={<div class="rounded-full text-md i-ri:download-2-fill" />}
          >
            <Trans
              key="instance.import"
              options={{
                defaultValue: "Import",
              }}
            />
          </Button>
        </div>
      </div>
      <div class="px-5 flex flex-col pb-5 gap-2 left-0 right-0 overflow-y-hidden absolute bottom-0 top-[90px]">
        <div class="flex flex-col gap-4 rounded-xl p-5 bg-darkSlate-700">
          <div class="flex justify-between items-center">
            <span class="flex gap-4">
              <div class="flex justify-center items-center rounded-xl bg-darkSlate-900 h-22 w-22">
                <img class="h-14" src={LogoDark} />
              </div>
              <div class="flex flex-col justify-center">
                <div class="flex flex-col gap-2">
                  <h2 class="m-0">
                    <Trans
                      key="instance.create_new_instance_title"
                      options={{
                        defaultValue: "New instance",
                      }}
                    />
                  </h2>
                  <p class="m-0 text-darkSlate-50">
                    <Trans
                      key="instance.create_new_instance_text"
                      options={{
                        defaultValue: "Create your own empty instance",
                      }}
                    />
                  </p>
                </div>
              </div>
            </span>
            <div class="flex gap-3">
              <Button
                variant="glow"
                onClick={() =>
                  modalsContext?.openModal({ name: "instanceCreation" })
                }
              >
                <span class="uppercase">
                  <Trans
                    key="instance.create_instance_btn"
                    options={{
                      defaultValue: "Create",
                    }}
                  />
                </span>
              </Button>
            </div>
          </div>
        </div>
        <Show when={modpacks.length > 0} fallback={<NoModpacks />}>
          <div
            ref={(el) => {
              containerRef = el;
            }}
            class="w-full h-full scrollbar-hide overflow-auto"
          >
            <div
              style={{
                height: `${rowVirtualizer.getTotalSize()}px`,
                width: "100%",
                position: "relative",
              }}
            >
              <For each={rowVirtualizer.getVirtualItems()}>
                {(virtualItem) => {
                  const isLoaderRow = () =>
                    virtualItem.index > modpacks.length - 1;
                  const modpack = () => modpacks[virtualItem.index];

                  const index =
                    (curseforgeSearch.data?.pagination &&
                      curseforgeSearch.data.pagination.index) ||
                    0;

                  const totalCount =
                    (curseforgeSearch.data?.pagination &&
                      curseforgeSearch.data.pagination.totalCount) ||
                    1;

                  const hasNextPage = index < totalCount;

                  return (
                    <div
                      data-index={virtualItem.index}
                      style={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        width: "100%",
                        height: `${virtualItem.size}px`,
                        transform: `translateY(${virtualItem.start}px)`,
                      }}
                    >
                      <div class="bg-darkSlate-700 rounded-xl">
                        <Switch fallback={<FetchingModpacks />}>
                          <Match when={!isLoaderRow() && modpack()}>
                            <Modpack modpack={modpack()} />
                          </Match>
                          <Match when={isLoaderRow() && !hasNextPage}>
                            <NoMoreModpacks />
                          </Match>
                        </Switch>
                      </div>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
        </Show>
        <Show when={curseforgeSearch.isError}>
          <ErrorFetchingModpacks error={curseforgeSearch.error} />
        </Show>
        {/* <Show when={curseforgeSearch.isFetching}>
          <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-full">
            <div class="flex justify-center items-center flex-col text-center">
              <p class="text-darkSlate-50 max-w-100">
                <Trans
                  key="instance.fetching_modpacks_text"
                  options={{
                    defaultValue: "Loading modpacks",
                  }}
                />
              </p>
              <Spinner />
            </div>
          </div>
        </Show> */}
      </div>
    </div>
  );
}
