import { Trans } from "@gd/i18n";
import { Button, Checkbox, Dropdown, Input } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { Show } from "solid-js";
import Tags from "./Tags";

export default function About() {
  const navigate = useNavigate();

  return (
    <div>
      <div class="flex flex-col bg-shade-8 z-10 transition-all duration-100 ease-in-out pt-6 px-6">
        <div class="flex justify-between items-center pb-4 flex-wrap gap-1">
          <Input
            placeholder="Type Here"
            icon={<div class="i-ri:search-line" />}
            class="w-full rounded-full text-shade-0"
            inputClass=""
          />
          <div class="flex gap-3 items-center">
            <p class="text-shade-0">
              <Trans
                key="sort_by"
                options={{
                  defaultValue: "Sort by:",
                }}
              />
            </p>
            <Dropdown
              options={[
                { label: "Popular", key: "popular" },
                { label: "Featured", key: "featured" },
                { label: "Author", key: "author" },
                { label: "Name", key: "name" },
                { label: "Total downloads", key: "downloads" },
              ]}
              value={"popular"}
              rounded
            />
          </div>
          <Button
            variant="outline"
            size="medium"
            icon={<div class="i-ri:download-2-fill text-lg" />}
          >
            <Trans
              key="import"
              options={{
                defaultValue: "Import",
              }}
            />
          </Button>
        </div>
        <div class="flex justify-between text-shade-0 z-10 mb-6">
          <Tags />
        </div>
      </div>
      <div class="h-full overflow-y-hidden">
        {/* <Show when={mods.length > 0} fallback={<NoMods />}>
          <For each={mods}>{(props) => <Mod mod={props} />}</For>
        </Show> */}
      </div>
    </div>
  );
}
