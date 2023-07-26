import { ModRowProps, getAuthors, isCurseForgeData } from "@/utils/Mods";
import { Accessor, For, Match, Show, Switch } from "solid-js";
import { Tooltip } from "@gd/ui";

type Props = {
  modProps: ModRowProps;
  isRowSmall: Accessor<boolean>;
};

const Authors = (props: Props) => {
  return (
    <Switch>
      <Match when={isCurseForgeData(props.modProps.data)}>
        <div class="text-sm whitespace-nowrap flex gap-2">
          <Switch>
            <Match when={!props.isRowSmall()}>
              <For each={getAuthors(props.modProps).slice(0, 2)}>
                {(author, i) => (
                  <>
                    <p class="m-0">{author?.name}</p>
                    <Show
                      when={
                        i() !==
                        getAuthors(props.modProps).slice(0, 2).length - 1
                      }
                    >
                      <span class="text-lightSlate-100">{"•"}</span>
                    </Show>
                  </>
                )}
              </For>
              <Show when={getAuthors(props.modProps).length > 2}>
                <Tooltip
                  content={
                    <div class="flex gap-2">
                      <For each={getAuthors(props.modProps).slice(3)}>
                        {(author) => <p class="m-0">{author?.name}</p>}
                      </For>
                    </div>
                  }
                >
                  <p class="m-0">{`+${
                    getAuthors(props.modProps).slice(3).length
                  }`}</p>
                </Tooltip>
              </Show>
            </Match>
            <Match when={props.isRowSmall()}>
              <p class="m-0">{getAuthors(props.modProps)[0]?.name}</p>
              <Show when={getAuthors(props.modProps).length - 1 > 0}>
                <Tooltip
                  content={
                    <div class="flex gap-2">
                      <For each={getAuthors(props.modProps).slice(1)}>
                        {(author) => <p class="m-0">{author?.name}</p>}
                      </For>
                    </div>
                  }
                >
                  <p class="m-0">{`+${
                    getAuthors(props.modProps).length - 1
                  }`}</p>
                </Tooltip>
              </Show>
            </Match>
          </Switch>
        </div>
      </Match>
      <Match when={!isCurseForgeData(props.modProps.data)}>
        <div>
          {!isCurseForgeData(props.modProps.data) &&
            props.modProps.data.modrinth.author}
        </div>
      </Match>
    </Switch>
  );
};

export default Authors;
