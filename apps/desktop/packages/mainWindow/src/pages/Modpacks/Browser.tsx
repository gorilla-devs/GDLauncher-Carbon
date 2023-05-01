import { Trans, useTransContext } from "@gd/i18n";
import { Button, Dropdown, Input, Spinner } from "@gd/ui";
import { For, Show, createEffect } from "solid-js";
import glassBlock from "/assets/images/icons/glassBlock.png";
import Modpack from "./Modpack";
import Tags from "./Tags";
import LogoDark from "/assets/images/logo-dark.svg";
import { useModal } from "@/managers/ModalsManager";
import { rspc } from "@/utils/rspcClient";
import { createStore } from "solid-js/store";
import { FEMod, FEModSearchParameters } from "@gd/core_module/bindings";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { RSPCError } from "@rspc/client";

const NoModpacks = () => {
  return (
    <div class="w-full flex h-full justify-center items-center min-h-90">
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
const FetchingModpacks = () => {
  return (
    <div class="w-full flex h-full justify-center items-center min-h-90">
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
          {props.error?.message}
        </p>
      </div>
    </div>
  );
};

export default function Browser() {
  const modalsContext = useModal();
  const [t] = useTransContext();
  const [modpacks, setModpacks] = createStore<FEMod[]>([]);
  const [query, setQuery] = createStore<FEModSearchParameters>({
    query: {
      categoryId: 0,
      classId: "modpacks",
      gameId: 432,
      gameVersion: "1.19.2",
      page: 1,
      modLoaderType: "forge",
      sortField: "popularity",
      sortOrder: "descending",
      pageSize: 20,
      slug: "",
      searchFilter: "",
      gameVersionTypeId: null,
      authorId: null,
      index: 0,
    },
  });

  const curseforgeSearch = rspc.createQuery(() => [
    "modplatforms.curseforgeSearch",
    query,
  ]);

  let containerRef: HTMLDivElement;

  createEffect(() => {
    if (curseforgeSearch.data?.data) {
      curseforgeSearch.data.data.forEach((element) => {
        setModpacks((prev) => [...prev, element]);
      });
    }
  });

  const rowVirtualizer = createVirtualizer({
    get count() {
      return modpacks.length + 1;
    },
    getScrollElement: () => containerRef,
    estimateSize: () => 230,
    overscan: 20,
  });

  createEffect(() => {
    const [lastItem] = [...rowVirtualizer.getVirtualItems()].reverse();
    if (!lastItem) return;

    if (lastItem.index >= modpacks.length - 1 && !curseforgeSearch.isFetching) {
      setQuery("query", (prev) => {
        return {
          ...prev,
          index: (prev.index as number) + 20 + 1,
        };
      });
    }
  });

  return (
    <div class="relative box-border max-h-full w-full">
      <div class="flex flex-col sticky top-0 left-0 right-0 bg-darkSlate-800 z-10 px-5 pt-5">
        <div class="flex items-center justify-between gap-3 pb-4 flex-wrap">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full text-darkSlate-50 rounded-full max-w-none flex-1"
            inputClass=""
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
              options={[
                { label: t("instance.sort_by_popular"), key: "popular" },
                { label: t("instance.sort_by_featured"), key: "featured" },
                { label: t("instance.sort_by_author"), key: "author" },
                { label: t("instance.sort_by_name"), key: "name" },
                {
                  label: t("instance.sort_by_total_downloads"),
                  key: "downloads",
                },
              ]}
              value={"popular"}
              rounded
            />
          </div>
          <Button
            variant="outline"
            size="medium"
            icon={<div class="rounded-full i-ri:download-2-fill text-md" />}
          >
            <Trans
              key="instance.import"
              options={{
                defaultValue: "Import",
              }}
            />
          </Button>
        </div>
        <div class="flex justify-between text-darkSlate-50 z-10 mb-6 max-w-150">
          <Tags />
        </div>
      </div>
      <div class="px-5 flex flex-col pb-5 gap-2 overflow-y-hidden max-h-[460px]">
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
              <Dropdown
                options={[
                  { label: "1.16.5", key: "1.16.5" },
                  { label: "1.16.4", key: "1.16.4" },
                  { label: "1.16.3", key: "1.16.3" },
                  { label: "1.16.2", key: "1.16.2" },
                ]}
                icon={<div class="i-ri:price-tag-3-fill" />}
                rounded
                bgColorClass="bg-darkSlate-400"
                value="1.16.2"
              />
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
            class="w-full overflow-auto h-[500px] scrollbar-hide"
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
                  const isLoaderRow = virtualItem.index > modpacks.length - 1;
                  const modpack = modpacks[virtualItem.index];
                  return (
                    <div
                      class="box-border py-2"
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
                        {isLoaderRow ? (
                          "Loading more..."
                        ) : (
                          <Modpack modpack={modpack} />
                        )}
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
        <Show when={curseforgeSearch.isFetching}>
          <FetchingModpacks />
        </Show>
      </div>
    </div>
  );
}
