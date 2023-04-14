/* eslint-disable i18next/no-literal-string */
import { Trans } from "@gd/i18n";
import { Dropdown, Tag } from "@gd/ui";
import { For } from "solid-js";

type TagType = {
  name: string;
  img: string;
};

type ModpackType = {
  img: string;
  name: string;
  tags: TagType[];
  author: string;
  download: number;
  lastUpdate: string;
  description: string;
};

type Props = { modpack: ModpackType };

const Modpack = (props: Props) => {
  return (
    <div class="p-5 flex flex-col gap-4 bg-shade-7 rounded-xl">
      <div class="flex gap-4">
        <img class="h-22 w-22" src={props.modpack.img} />
        <div class="flex flex-col justify-between">
          <div class="flex justify-between">
            <h2 class="mt-0 mb-1">{props.modpack.name}</h2>
            <div class="flex gap-4 items-center">
              <div class="flex gap-2 items-center">
                <i class="i-ri:time-fill text-shade-1" />
                <div class="text-sm">1d ago</div>
              </div>
              <div class="flex gap-2 items-center">
                <i class="text-shade-1 i-ri:download-fill" />
                <div class="text-sm">4.8M</div>
              </div>
              <div class="flex gap-2 items-center">
                <i class="i-ri:user-fill text-shade-1" />
                <div class="text-sm">ATMTeam</div>
              </div>
            </div>
          </div>
          <p class="m-0 text-shade-0 text-sm">{props.modpack.description}</p>
        </div>
      </div>
      <div class="flex justify-between items-center gap-3">
        <div class="flex gap-2">
          <For each={props.modpack.tags}>
            {(tag) => <Tag name={tag.name} img={tag.img} type="fixed" />}
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
              key="download"
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
