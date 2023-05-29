import { formatDownloadCount, truncateText } from "@/utils/helpers";
import { FEMod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Button, Dropdown, Tag } from "@gd/ui";
import { format } from "date-fns";
import { For } from "solid-js";
import { useModal } from "../..";

type Props = { mod: FEMod };

const Mod = (props: Props) => {
  const modalsContext = useModal();

  const latestFIlesIndexes = () => props.mod.latestFilesIndexes;

  const mappedVersions = () =>
    latestFIlesIndexes().map((version) => ({
      key: version.gameVersion,
      label: version.gameVersion,
    }));

  return (
    <div class="flex flex-col gap-4 p-5 bg-darkSlate-700 rounded-2xl max-h-60">
      <div class="flex gap-4">
        <img
          class="rounded-xl select-none h-30 w-30"
          src={props.mod.logo.thumbnailUrl}
        />
        <div class="flex flex-col gap-2">
          <div class="flex flex-col justify-between">
            <h2 class="mt-0 text-ellipsis overflow-hidden whitespace-nowrap mb-1 max-w-92">
              {props.mod.name}
            </h2>
            <div class="flex gap-4 items-center">
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:time-fill" />
                <div class="whitespace-nowrap text-sm">
                  {format(new Date(props.mod.dateCreated).getTime(), "P")}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:download-fill" />
                <div class="text-sm whitespace-nowrap">
                  {formatDownloadCount(props.mod.downloadCount)}
                </div>
              </div>
              <div class="flex gap-2 items-center text-darkSlate-100">
                <i class="text-darkSlate-100 i-ri:user-fill" />
                <div class="text-sm whitespace-nowrap flex gap-2 max-w-52 overflow-x-auto">
                  <For each={props.mod.authors}>
                    {(author) => <p class="m-0">{author.name}</p>}
                  </For>
                </div>
              </div>
            </div>
          </div>
          <p class="m-0 text-sm text-darkSlate-50 overflow-hidden text-ellipsis max-h-15">
            {truncateText(props.mod?.summary, 137)}
          </p>
        </div>
      </div>
      <div class="flex justify-between items-center gap-3">
        <div class="flex gap-2 overflow-x-auto max-w-100 scrollbar-hide">
          <For each={props.mod.categories}>
            {(tag) => <Tag name={tag.name} img={tag.iconUrl} type="fixed" />}
          </For>
        </div>
        <div class="flex gap-3">
          <Button
            variant="outline"
            onClick={() =>
              modalsContext?.openModal(
                {
                  name: "modDetails",
                },
                { mod: props.mod }
              )
            }
          >
            <Trans
              key="instance.explore_modpack"
              options={{
                defaultValue: "Explore",
              }}
            />
          </Button>
          {/* <Dropdown.button
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
          </Dropdown.button> */}
          <Dropdown.button
            options={mappedVersions()}
            rounded
            value={mappedVersions()[0]?.key}
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

export default Mod;
