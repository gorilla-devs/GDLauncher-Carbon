import {
  ModRowProps,
  getAuthors,
  getCategories,
  getDateModification,
  getDownloads,
  getLatestVersion,
  getLogoUrl,
  getName,
  getSummary,
  getWebsiteUrl,
  isCurseForgeData
} from "@/utils/mods";
import { formatDownloadCount } from "@/utils/helpers";
import { CategoryIcon } from "@/utils/instances";
import {
  CFFECategory,
  CFFEModAuthor,
  MRFECategoriesResponse,
  MRFECategory,
  MRFEProjectSearchResult
} from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Tag } from "@gd/ui";
import { formatDistanceToNowStrict } from "date-fns";
import { For, Match, Show, Switch } from "solid-js";

const Authors = (props: { data: ModRowProps }) => {
  return (
    <div class="flex gap-2 max-w-full flex-wrap scrollbar-hide">
      <Switch>
        <Match when={isCurseForgeData(props.data.data)}>
          <For each={getAuthors(props.data)}>
            {(author, i) => (
              <>
                <p class="m-0 text-sm">{(author as CFFEModAuthor)?.name}</p>
                <Show when={i() !== getAuthors(props.data).length - 1}>
                  <span class="text-lightSlate-100">{"•"}</span>
                </Show>
              </>
            )}
          </For>
        </Match>
        <Match when={!isCurseForgeData(props.data.data)}>
          <p class="m-0 text-sm">
            {
              (props.data.data as { modrinth: MRFEProjectSearchResult })
                .modrinth.author
            }
          </p>
        </Match>
      </Switch>
    </div>
  );
};

const OverviewPopover = (props: {
  data: ModRowProps;
  modrinthCategories: MRFECategoriesResponse | undefined;
}) => {
  return (
    <div class="relative flex flex-col overflow-hidden pb-4 w-70">
      <Show when={getWebsiteUrl(props.data)}>
        <div
          class="rounded-lg bg-darkSlate-900 cursor-pointer w-6 h-6"
          onClick={() => {
            const url = getWebsiteUrl(props.data);
            if (url) window.openExternalLink(url);
          }}
        >
          <div class="w-4 h-4 text-lightSlate-500 hover:text-lightSlate-50 transition-color ease-in-out transition-100 absolute i-ri:external-link-line top-4 right-4 z-30" />
        </div>
      </Show>
      <h4 class="text-xl z-30 text-lightSlate-100 px-4 mb-2">
        {getName(props.data)}
      </h4>
      <div class="absolute top-0 bottom-0 right-0 left-0 z-20 bg-gradient-to-t from-darkSlate-900 from-70%" />
      <div class="absolute top-0 bottom-0 right-0 bottom-0 left-0 from-darkSlate-900 z-20 bg-gradient-to-l" />
      <Show when={getLogoUrl(props.data)}>
        <img
          class="absolute right-0 top-0 bottom-0 select-none h-full w-full z-10 blur-sm"
          src={getLogoUrl(props.data) as string}
        />
      </Show>
      <div class="px-4 z-30">
        <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis">
          {getSummary(props.data)}
        </p>
        <div class="flex gap-2 scrollbar-hide mt-4">
          <For each={getCategories(props.data)}>
            {(tag) => (
              <Tag
                img={
                  isCurseForgeData(props.data.data) ? (
                    (tag as CFFECategory).iconUrl
                  ) : (
                    <div>
                      <Switch>
                        <Match
                          when={props.modrinthCategories?.find(
                            (category) => category.name === tag
                          )}
                        >
                          <CategoryIcon
                            category={
                              props.modrinthCategories?.find(
                                (category) => category.name === tag
                              ) as MRFECategory
                            }
                          />
                        </Match>
                      </Switch>
                    </div>
                  )
                }
                type="fixed"
              />
            )}
          </For>
        </div>
        <div class="flex flex-col gap-2 items-start mt-4">
          <div class="flex gap-2 items-start text-darkSlate-100">
            <span class="flex gap-2 items-center">
              <div class="text-lightSlate-100 w-4 h-4 i-ri:user-fill" />
              <p class="m-0 text-lightSlate-100 text-sm">
                <Trans key="modpack.authors" />
              </p>
            </span>
            <Authors data={props.data} />
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <div class="text-lightSlate-100 i-ri:time-fill" />
            <p class="m-0 text-lightSlate-100 text-sm">
              <Trans key="modpack.last_updated" />
            </p>
            <div class="whitespace-nowrap text-sm">
              <Trans
                key="modpack.last_updated_time"
                options={{
                  time: formatDistanceToNowStrict(
                    new Date(getDateModification(props.data)).getTime()
                  )
                }}
              />
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <div class="text-lightSlate-100 i-ri:download-fill" />
            <p class="m-0 text-lightSlate-100 text-sm">
              <Trans key="modpack.total_download" />
            </p>
            <div class="text-sm whitespace-nowrap">
              {formatDownloadCount(getDownloads(props.data))}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <div class="text-lightSlate-100 i-ri:gamepad-fill" />
            <p class="m-0 text-lightSlate-100 text-sm">
              <Trans key="modpack.mcVersion" />
            </p>
            <div class="flex flex-wrap gap-2 scrollbar-hide max-w-full text-sm">
              {getLatestVersion(props.data)}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default OverviewPopover;
