/* eslint-disable i18next/no-literal-string */
import { formatDownloadCount } from "@/utils/helpers";
import { FEMod } from "@gd/core_module/bindings";
import { Trans } from "@gd/i18n";
import { Dropdown, Tag } from "@gd/ui";
import { For } from "solid-js";

type Props = { modpack: FEMod };

const Modpack = (props: Props) => {
  return (
    <div class="p-5 flex flex-col gap-4 bg-darkSlate-700 rounded-xl max-h-96">
      <div class="flex gap-4">
        <img
          class="h-30 rounded-xl w-30"
          src={props.modpack.logo.thumbnailUrl}
        />
        <div class="flex flex-col gap-2">
          <div class="flex flex-col justify-between">
            <h2 class="mt-0 mb-1 text-ellipsis overflow-hidden whitespace-nowrap max-w-92">
              {props.modpack.name}
            </h2>
            <div class="flex gap-4 items-center">
              <div class="flex gap-2 items-center">
                <i class="i-ri:time-fill text-darkSlate-100" />
                <div class="text-sm whitespace-nowrap">1d ago</div>
              </div>
              <div class="flex gap-2 items-center">
                <i class="text-darkSlate-100 i-ri:download-fill" />
                <div class="text-sm whitespace-nowrap">
                  {formatDownloadCount(props.modpack.downloadCount)}
                </div>
              </div>
              <div class="flex gap-2 items-center">
                <i class="i-ri:user-fill text-darkSlate-100" />
                <div class="text-sm whitespace-nowrap flex gap-2">
                  <For each={props.modpack.authors}>
                    {(author) => <p class="m-0">{author.name}</p>}
                  </For>
                </div>
              </div>
            </div>
          </div>
          <p class="m-0 text-darkSlate-50 text-sm h-10">
            {props.modpack.summary}
          </p>
        </div>
      </div>
      <div class="flex justify-between items-center gap-3">
        <div class="flex gap-2 max-w-100 overflow-x-auto">
          <For each={props.modpack.categories}>
            {(tag) => <Tag name={tag.name} img={tag.avatarUrl} type="fixed" />}
          </For>
        </div>
        <div class="flex gap-3">
          <Dropdown.button
            onChange={() => {}}
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
