import { For } from "solid-js";
import { Button, Tag } from "@gd/ui";
import { Trans } from "@gd/i18n";

type Tagtype = {
  name: string;
  img: string;
};

type Props = {
  tags: Tagtype[];
  onClose: (_name: string) => void;
  onClearAll: () => void;
};

const Tags = (props: Props) => {
  return (
    <div class="flex gap-2 w-full items-center space-between">
      <div class="scrollbar-hide flex flex-1 gap-2 w-full overflow-x-auto flex-1 grow">
        <For each={props.tags}>
          {(tag) => (
            <Tag name={tag.name} img={tag.img} onClose={props.onClose} />
          )}
        </For>
      </div>
      <Button
        class="h-8"
        type="secondary"
        textColor="text-red-500"
        rounded={false}
        onClick={() => props.onClearAll()}
      >
        <Trans
          key="instance.clear_filters_modpacks"
          options={{
            defaultValue: "Clear filters"
          }}
        />
      </Button>
    </div>
  );
};

export default Tags;
