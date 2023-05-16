import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input, Skeleton, Spinner } from "@gd/ui";
import { For, Match, Show, Switch, createEffect, onMount } from "solid-js";
import Modpack from "./Modpack";
import LogoDark from "/assets/images/logo-dark.svg";
import { useModal } from "@/managers/ModalsManager";
import { FEModSearchSortField } from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { useInfiniteQuery } from ".";
import { mappedMcVersions } from "@/utils/mcVersion";

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
    <div class="flex flex-col justify-center items-center gap-4 p-5 rounded-xl h-56">
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

  const infiniteQuery = useInfiniteQuery();

  const modpacks = () =>
    infiniteQuery?.infiniteQuery.data
      ? infiniteQuery?.infiniteQuery.data.pages.flatMap((d) => d.data)
      : [];

  const allVirtualRows = () => infiniteQuery?.rowVirtualizer.getVirtualItems();

  const lastItem = () => allVirtualRows()[allVirtualRows().length - 1];
  createEffect(() => {
    if (!lastItem() || lastItem().index === infiniteQuery?.query.query.index) {
      return;
    }

    const lastItemIndex = infiniteQuery?.infiniteQuery.hasNextPage
      ? lastItem().index - 1
      : lastItem().index;

    if (
      lastItemIndex >= modpacks().length - 1 &&
      infiniteQuery?.infiniteQuery.hasNextPage &&
      !infiniteQuery.infiniteQuery.isFetchingNextPage
    ) {
      infiniteQuery.infiniteQuery.fetchNextPage();
    }
  });

  onMount(() => {
    if (modpacks().length > 0 && !infiniteQuery?.infiniteQuery.isInitialLoading)
      infiniteQuery?.resetList();
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
              infiniteQuery?.setQuery({ searchFilter: target.value });
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
                infiniteQuery?.setQuery({
                  sortField: val.key as FEModSearchSortField,
                });
              }}
              value={0}
              rounded
            />
            <Show when={mappedMcVersions().length > 0}>
              <Dropdown
                options={mappedMcVersions()}
                icon={<div class="i-ri:price-tag-3-fill" />}
                rounded
                bgColorClass="bg-darkSlate-400"
                textColorClass="text-white"
                value={mappedMcVersions()[0].key}
                onChange={(val) => {
                  infiniteQuery?.setQuery({ gameVersion: val.key as string });
                }}
              />
            </Show>
            <Show when={mappedMcVersions().length === 0}>
              <Skeleton.select />
            </Show>
          </div>
          <div
            class="cursor-pointer text-2xl"
            classList={{
              "i-ri:sort-asc":
                infiniteQuery?.query.query.sortOrder === "ascending",
              "i-ri:sort-desc":
                infiniteQuery?.query.query.sortOrder === "descending",
            }}
            onClick={() => {
              const isAsc =
                infiniteQuery?.query.query.sortOrder === "ascending";
              infiniteQuery?.setQuery({
                sortOrder: isAsc ? "descending" : "ascending",
              });
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
        <Switch>
          <Match
            when={
              modpacks().length > 0 &&
              !infiniteQuery?.infiniteQuery.isInitialLoading
            }
          >
            <div
              class="w-full h-full scrollbar-hide overflow-auto"
              ref={(el) => {
                infiniteQuery?.setParentRef(el);
              }}
            >
              <div
                style={{
                  height: `${infiniteQuery?.rowVirtualizer.getTotalSize()}px`,
                  width: "100%",
                  position: "relative",
                }}
              >
                <For each={allVirtualRows()}>
                  {(virtualItem) => {
                    const isLoaderRow = () =>
                      virtualItem.index > modpacks().length - 1;
                    const modpack = () => modpacks()[virtualItem.index];

                    const hasNextPage = () =>
                      infiniteQuery?.infiniteQuery.hasNextPage;

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
                        <div>
                          <Switch
                            fallback={
                              <div>
                                <FetchingModpacks />
                              </div>
                            }
                          >
                            <Match when={!isLoaderRow() && modpack()}>
                              <Modpack modpack={modpack()} />
                            </Match>
                            <Match when={isLoaderRow() && !hasNextPage()}>
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
          </Match>
          <Match
            when={
              modpacks().length === 0 &&
              infiniteQuery?.infiniteQuery.isFetching &&
              infiniteQuery?.infiniteQuery.isInitialLoading
            }
          >
            <Skeleton.modpacksList />
          </Match>
          <Match when={infiniteQuery?.infiniteQuery.isError}>
            <ErrorFetchingModpacks
              error={infiniteQuery?.infiniteQuery.error as RSPCError | null}
            />
          </Match>
        </Switch>
      </div>
    </div>
  );
}
