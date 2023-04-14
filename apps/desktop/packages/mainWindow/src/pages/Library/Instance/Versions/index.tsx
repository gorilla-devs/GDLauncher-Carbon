import { Button } from "@gd/ui";
import { For, Show } from "solid-js";
import { Trans } from "@gd/i18n";
import Version from "./Version";
import glassBlock from "/assets/images/icons/glassBlock.png";

type VersionType = {
  title: string;
  mcversion: string;
  modloader: string;
  date: string;
  stable: string;
  isActive: boolean;
};

const versions: VersionType[] = [
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "stable",
    isActive: true,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "stable",
    isActive: false,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "stable",
    isActive: false,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "stable",
    isActive: false,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "stable",
    isActive: false,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "stable",
    isActive: false,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "beta",
    isActive: false,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "stable",
    isActive: false,
  },
  {
    title: "Mods1",
    mcversion: "1.19.2",
    modloader: "fabric",
    date: "2023-01-31T09:20:53.513Z",
    stable: "alpha",
    isActive: false,
  },
];

const NoVersions = () => {
  return (
    <div class="h-full min-h-90 w-full flex justify-center items-center">
      <div class="flex flex-col justify-center items-center text-center">
        <img src={glassBlock} class="w-16 h-16" />
        <p class="text-darkSlate-50 max-w-100">
          <Trans
            key="instance.no_resource_packs_text"
            options={{
              defaultValue:
                "At the moment this modpack does not contain resource packs, but you can add packs yourself from your folder",
            }}
          />
        </p>
        <Button variant="outline" size="medium">
          <Trans
            key="instance.add_pack"
            options={{
              defaultValue: "+ Add pack",
            }}
          />
        </Button>
      </div>
    </div>
  );
};

const Versions = () => {
  return (
    <div>
      <div class="h-full overflow-y-hidden">
        <Show when={versions.length > 0} fallback={<NoVersions />}>
          <For each={versions}>{(props) => <Version version={props} />}</For>
        </Show>
      </div>
    </div>
  );
};

export default Versions;
