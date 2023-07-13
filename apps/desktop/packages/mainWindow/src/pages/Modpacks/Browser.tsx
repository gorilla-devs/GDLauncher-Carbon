import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input, Skeleton, Spinner } from "@gd/ui";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  onMount,
} from "solid-js";
import { FEModSearchSortField } from "@gd/core_module/bindings";
import { RSPCError } from "@rspc/client";
import { useInfiniteModpacksQuery } from ".";
import { mappedMcVersions } from "@/utils/mcVersion";
import { CurseForgeSortFields } from "@/utils/constants";
import { rspc } from "@/utils/rspcClient";
import { setScrollTop } from "@/utils/browser";
import skull from "/assets/images/icons/skull.png";
import ModRow from "@/components/ModRow";
import { useModal } from "@/managers/ModalsManager";

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

const NoModpacksAvailable = () => {
  return (
    <div class="flex flex-col justify-center items-center gap-4 p-5 bg-darkSlate-700 rounded-xl h-100">
      <div class="flex justify-center items-center flex-col text-center">
        <img src={skull} class="w-16 h-16" />

        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.fetching_no_modpacks_available"
            options={{
              defaultValue: "No modpacks available",
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

export default function Browser() {
  const [t] = useTransContext();
  const modalsContext = useModal();

  const defaultGroup = rspc.createQuery(() => ["instance.getDefaultGroup"]);

  const infiniteQuery = useInfiniteModpacksQuery();

  const modpacks = () => infiniteQuery.allRows();

  const allVirtualRows = () => infiniteQuery.rowVirtualizer.getVirtualItems();

  const lastItem = () => allVirtualRows()[allVirtualRows().length - 1];
  createEffect(() => {
    if (!lastItem() || lastItem().index === infiniteQuery?.query.index) {
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

  const [headerHeight, setHeaderHeight] = createSignal(90);

  let containrRef: HTMLDivElement;
  let resizeObserver: ResizeObserver;

  onMount(() => {
    resizeObserver = new ResizeObserver((entries) => {
      window.requestAnimationFrame(() => {
        setHeaderHeight(entries[0].target.getBoundingClientRect().height);
      });
    });

    resizeObserver.observe(containrRef);
  });

  onCleanup(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
  });

  return (
    <div class="box-border h-full w-full relative">
      <div
        ref={(el) => (containrRef = el)}
        class="flex flex-col bg-darkSlate-800 pt-5 z-10 px-5"
      >
        <div class="flex items-center justify-between gap-3 pb-4 flex-wrap">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full text-darkSlate-50 rounded-full flex-1 max-w-none"
            onInput={(e) => {
              const target = e.target as HTMLInputElement;
              infiniteQuery?.setQuery({ searchQuery: target.value });
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
              options={CurseForgeSortFields.map((field) => ({
                label: t(`instance.sort_by_${field}`),
                key: field,
              }))}
              onChange={(val) => {
                // infiniteQuery?.setQuery({
                //   sortIndex: val.key as FEModSearchSortField,
                // });

                infiniteQuery?.setQuery({
                  sortIndex: {
                    curseForge: val.key as FEModSearchSortField,
                  },
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
                value={mappedMcVersions()[0].key}
                onChange={(val) => {
                  infiniteQuery?.setQuery({
                    gameVersions: [val.key as string],
                  });
                }}
              />
            </Show>
            <Show when={mappedMcVersions().length === 0}>
              <Skeleton.select />
            </Show>
          </div>
          <Button
            type="outline"
            onClick={() => {
              modalsContext?.openModal({
                name: "instanceCreation",
              });
            }}
          >
            <Trans
              key="sidebar.plus_add_instance"
              options={{
                defaultValue: "+ Add Instance",
              }}
            />
          </Button>
          <div
            class="cursor-pointer text-2xl"
            classList={{
              "i-ri:sort-asc": infiniteQuery?.query.sortOrder === "ascending",
              "i-ri:sort-desc": infiniteQuery?.query.sortOrder === "descending",
            }}
            onClick={() => {
              const isAsc = infiniteQuery?.query.sortOrder === "ascending";
              infiniteQuery?.setQuery({
                sortOrder: isAsc ? "descending" : "ascending",
              });
            }}
          />
          {/* <Button
            type="outline"
            size="medium"
            icon={<div class="rounded-full text-md i-ri:download-2-fill" />}
          >
            <Trans
              key="instance.import"
              options={{
                defaultValue: "Import",
              }}
            />
          </Button> */}
        </div>
      </div>
      <div
        class="flex flex-col pb-5 gap-2 left-0 right-0 absolute bottom-0 overflow-y-hidden"
        style={{
          top: `${headerHeight()}px`,
        }}
      >
        <Switch>
          <Match
            when={
              modpacks().length > 0 &&
              !infiniteQuery?.infiniteQuery.isInitialLoading
            }
          >
            <div
              class="h-full overflow-y-auto rounded-xl overflow-x-hidden pr-2 ml-5"
              ref={(el) => {
                infiniteQuery.setParentRef(el);
              }}
              onScroll={(e) => {
                setScrollTop(e.target.scrollTop);
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
                              <ModRow
                                type="Modpack"
                                data={modpack()}
                                defaultGroup={defaultGroup.data}
                              />
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
              infiniteQuery?.infiniteQuery.isLoading &&
              infiniteQuery?.infiniteQuery.isInitialLoading
            }
          >
            <div class="m-x-5">
              <Skeleton.modpacksList />
            </div>
          </Match>
          <Match
            when={
              modpacks().length === 0 &&
              !infiniteQuery?.infiniteQuery.isLoading &&
              !infiniteQuery?.infiniteQuery.isInitialLoading
            }
          >
            <NoModpacksAvailable />
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
