import { useGDNavigate } from "@/managers/NavigationManager";
import { formatDownloadCount } from "@/utils/helpers";
import { FEMod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Tag } from "@gd/ui";
import { format } from "date-fns";
import { For } from "solid-js";

type Props = { modpack: FEMod };

const truncateText = (text: string, maxLength: number): string => {
  if (text.length <= maxLength) {
    return text;
  }

  return text.slice(0, maxLength) + "...";
};

const Modpack = (props: Props) => {
  const navigate = useGDNavigate();
  return (
    <div class="flex flex-col gap-4 p-5 bg-darkSlate-700 rounded-2xl max-h-60">
      <div class="flex gap-4">
        <img
          class="rounded-xl select-none h-30 w-30"
          src={props.modpack.logo.thumbnailUrl}
        />
        <div class="flex flex-col gap-2">
          <div class="flex flex-col justify-between">
            <h2 class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 max-w-92">
              {props.modpack.name}
            </h2>
            <div class="flex gap-4 items-center">
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:time-fill" />
                <div class="whitespace-nowrap text-sm">
                  {format(new Date(props.modpack.dateCreated).getTime(), "P")}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:download-fill" />
                <div class="text-sm whitespace-nowrap">
                  {formatDownloadCount(props.modpack.downloadCount)}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:user-fill" />
                <div class="text-sm whitespace-nowrap flex gap-2 max-w-52 overflow-x-auto">
                  <For each={props.modpack.authors}>
                    {(author) => <p class="m-0">{author.name}</p>}
                  </For>
                </div>
              </div>
            </div>
          </div>
          <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis max-h-15">
            {truncateText(props.modpack?.summary, 137)}
          </p>
        </div>
      </div>
      <div class="flex justify-between items-center gap-3">
        <div class="flex gap-2 overflow-x-auto max-w-100 scrollbar-hide">
          <For each={props.modpack.categories}>
            {(tag) => <Tag name={tag.name} img={tag.iconUrl} type="fixed" />}
          </For>
        </div>
        <div class="flex gap-3">
          <Button
            type="outline"
            onClick={() => navigate(`/modpacks/${props.modpack.id}`)}
          >
            <Trans
              key="instance.explore_modpack"
              options={{
                defaultValue: "Explore",
              }}
            />
          </Button>
          <Dropdown.button
            options={[
              { label: "1.16.5", key: "1.16.5" },
              { label: "1.16.4", key: "1.16.4" },
              { label: "1.16.3", key: "1.16.3" },
              { label: "1.16.2", key: "1.16.2" },
            ]}
            rounded
            value="1.16.2"
          >
            <Trans
              key="instance.download_modpacks"
              options={{
                defaultValue: "Download",
              }}
            />
          </Dropdown.button>
        </div>
      </div>
    </div>
  );
};

export default Modpack;
