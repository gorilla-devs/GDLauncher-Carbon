import { formatDownloadCount } from "@/utils/helpers";
import { FEMod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Tag } from "@gd/ui";
import { formatDistanceToNowStrict } from "date-fns";
import { For, Show } from "solid-js";

const OverviewPopover = (props: { data: FEMod }) => {
  return (
    <div class="relative flex flex-col overflow-hidden w-70 pb-4">
      <Show when={props.data.links.websiteUrl}>
        <div
          class="rounded-lg bg-darkSlate-900 cursor-pointer w-6 h-6"
          onClick={() =>
            window.openExternalLink(props.data.links.websiteUrl as string)
          }
        >
          <div class="w-4 h-4 text-lightSlate-100 absolute i-ri:external-link-line z-30 top-4 right-4" />
        </div>
      </Show>
      <h4 class="text-xl z-30 text-lightSlate-100 px-4 mb-2">
        {props.data?.name}
      </h4>
      <div class="absolute top-0 bottom-0 right-0 left-0 z-20 bg-gradient-to-t from-darkSlate-900 from-70%" />
      <div class="absolute top-0 bottom-0 right-0 bottom-0 left-0 bg-gradient-to-l from-darkSlate-900 z-20" />
      <img
        class="absolute right-0 top-0 bottom-0 select-none h-full w-full z-10 blur-sm"
        src={props.data.logo.thumbnailUrl}
      />
      <div class="px-4 z-30">
        <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis">
          {props.data?.summary}
        </p>
        <div class="flex gap-2 scrollbar-hide mt-4">
          <For each={props.data.categories}>
            {(tag) => <Tag img={tag.iconUrl} type="fixed" size="small" />}
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
            <div class="flex flex-wrap gap-2 scrollbar-hide max-w-full">
              <For each={props.data.authors}>
                {(author, i) => (
                  <>
                    <p class="m-0 text-sm">{author?.name}</p>
                    <Show when={i() !== props.data.authors.length - 1}>
                      <span class="text-lightSlate-100">{"•"}</span>
                    </Show>
                  </>
                )}
              </For>
            </div>
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
                    new Date(props.data.dateModified).getTime()
                  ),
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
              {formatDownloadCount(props.data.downloadCount)}
            </div>
          </div>
          <div class="flex gap-2 items-center text-darkSlate-100">
            <div class="text-lightSlate-100 i-ri:gamepad-fill" />
            <p class="m-0 text-lightSlate-100 text-sm">
              <Trans key="modpack.mcVersion" />
            </p>
            <div class="flex flex-wrap gap-2 scrollbar-hide max-w-full text-sm">
              {props.data.latestFilesIndexes[0].gameVersion}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default OverviewPopover;
