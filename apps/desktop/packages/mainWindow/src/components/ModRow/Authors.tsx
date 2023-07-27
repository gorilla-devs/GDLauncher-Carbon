import { ModRowProps, isCurseForgeData } from "@/utils/Mods";
import { Accessor, For, Match, Show, Switch } from "solid-js";
import { Tooltip } from "@gd/ui";
import { CFFEModAuthor, FEUnifiedSearchResult } from "@gd/core_module/bindings";

type Props = {
  modProps: ModRowProps | FEUnifiedSearchResult;
  isRowSmall?: Accessor<boolean>;
};

export const getAuthors = (prop: ModRowProps | FEUnifiedSearchResult) => {
  const isModRow = "data" in prop;
  if (isModRow) {
    if (isCurseForgeData(prop.data)) {
      return prop.data.curseforge.authors;
    } else return prop.data.modrinth.author;
  } else {
    if (isCurseForgeData(prop)) {
      return prop.curseforge.authors;
    } else return prop.modrinth.author;
  }
};

const Authors = (props: Props) => {
  const isModRow = () => "data" in props;
  const modProps = () =>
    isModRow()
      ? (props.modProps as ModRowProps).data
      : (props.modProps as FEUnifiedSearchResult);

  return (
    <Switch>
      <Match when={isCurseForgeData(modProps())}>
        <div class="text-sm whitespace-nowrap flex gap-2">
          <Switch>
            <Match when={!props?.isRowSmall?.()}>
              <For
                each={(getAuthors(props.modProps) as CFFEModAuthor[]).slice(
                  0,
                  2
                )}
              >
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
                      <For
                        each={(
                          getAuthors(props.modProps) as CFFEModAuthor[]
                        ).slice(3)}
                      >
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
            <Match when={props?.isRowSmall?.()}>
              <p class="m-0">
                {(getAuthors(props.modProps) as CFFEModAuthor[])[0]?.name}
              </p>
              <Show when={getAuthors(props.modProps).length - 1 > 0}>
                <Tooltip
                  content={
                    <div class="flex gap-2">
                      <For
                        each={(
                          getAuthors(props.modProps) as CFFEModAuthor[]
                        ).slice(1)}
                      >
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
      <Match when={!isCurseForgeData(modProps())}>
        <div>
          {!isCurseForgeData(modProps()) &&
            (getAuthors(props.modProps) as string)}
        </div>
      </Match>
    </Switch>
  );
};

export default Authors;
