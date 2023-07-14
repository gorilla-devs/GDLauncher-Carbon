import { ModRowProps, getCategories, isCurseForgeData } from "@/utils/Mods";
import { Accessor, For, Match, Show, Switch, createEffect } from "solid-js";
import { Tag, Tooltip } from "@gd/ui";
import { FECategory } from "@gd/core_module/bindings";

type Props = {
  modProps: ModRowProps;
  isRowSmall: Accessor<boolean>;
};

const Categories = (props: Props) => {
  return (
    <div class="flex gap-2 scrollbar-hide">
      <Switch>
        <Match when={!props.isRowSmall()}>
          <For each={getCategories(props.modProps)}>
            {(tag) => (
              <Tooltip
                content={
                  isCurseForgeData(props.modProps.data)
                    ? (tag as FECategory).name
                    : (tag as string)
                }
              >
                <Tag
                  img={
                    isCurseForgeData(props.modProps.data)
                      ? (tag as FECategory).iconUrl
                      : null
                  }
                  name={
                    !isCurseForgeData(props.modProps.data)
                      ? (tag as string)
                      : ""
                  }
                  type="fixed"
                />
              </Tooltip>
            )}
          </For>
        </Match>
        <Match when={props.isRowSmall()}>
          <Tooltip
            content={
              isCurseForgeData(props.modProps.data)
                ? (getCategories(props.modProps)?.[0] as FECategory)?.name
                : (getCategories(props.modProps)?.[0] as string)
            }
          >
            <Tag
              img={
                isCurseForgeData(props.modProps.data)
                  ? (getCategories(props.modProps)?.[0] as FECategory)?.iconUrl
                  : null
              }
              name={
                !isCurseForgeData(props.modProps.data)
                  ? (getCategories(props.modProps)?.[0] as string)
                  : ""
              }
              type="fixed"
            />
          </Tooltip>
          <Show when={getCategories(props.modProps).length - 1 > 0}>
            <Tooltip
              content={
                <div class="flex">
                  <Switch>
                    <Match when={isCurseForgeData(props.modProps.data)}>
                      <For each={getCategories(props.modProps).slice(1)}>
                        {(tag) => (
                          <Tag
                            img={(tag as FECategory).iconUrl}
                            name={(tag as FECategory).name}
                            type="fixed"
                          />
                        )}
                      </For>
                    </Match>
                    <Match when={!isCurseForgeData(props.modProps.data)}>
                      <For each={getCategories(props.modProps).slice(1)}>
                        {(tag) => <Tag name={tag as string} type="fixed" />}
                      </For>
                    </Match>
                  </Switch>
                </div>
              }
            >
              <Tag
                name={`+${getCategories(props.modProps).length - 1}`}
                type="fixed"
              />
            </Tooltip>
          </Show>
        </Match>
      </Switch>
    </div>
  );
};

export default Categories;
