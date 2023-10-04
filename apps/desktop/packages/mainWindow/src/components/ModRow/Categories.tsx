import { ModRowProps, getCategories, isCurseForgeData } from "@/utils/mods";
import { Accessor, For, Match, Show, Switch } from "solid-js";
import { Tag, Tooltip } from "@gd/ui";
import {
  CFFECategory,
  MRFECategoriesResponse,
  MRFECategory
} from "@gd/core_module/bindings";
import { CategoryIcon } from "@/utils/instances";
import { capitalize } from "@/utils/helpers";

type Props = {
  modProps: ModRowProps;
  isRowSmall: Accessor<boolean>;
  modrinthCategories: MRFECategoriesResponse | undefined;
};

const Categories = (props: Props) => {
  return (
    <div class="flex gap-2 scrollbar-hide">
      <Switch>
        <Match when={!props.isRowSmall()}>
          <For each={getCategories(props.modProps)}>
            {(tag) => {
              const modrinthCategory = () =>
                props.modrinthCategories?.find(
                  (category) => category.name === tag
                );
              return (
                <Tooltip
                  content={
                    isCurseForgeData(props.modProps.data)
                      ? (tag as CFFECategory).name
                      : capitalize(tag as string)
                  }
                >
                  <Tag
                    img={
                      isCurseForgeData(props.modProps.data) ? (
                        (tag as CFFECategory).iconUrl
                      ) : (
                        <div>
                          <Switch fallback={capitalize(tag as string)}>
                            <Match when={modrinthCategory()}>
                              <CategoryIcon
                                category={modrinthCategory() as MRFECategory}
                              />
                            </Match>
                          </Switch>
                        </div>
                      )
                    }
                    type="fixed"
                  />
                </Tooltip>
              );
            }}
          </For>
        </Match>
        <Match when={props.isRowSmall()}>
          <Tooltip
            content={
              isCurseForgeData(props.modProps.data)
                ? (getCategories(props.modProps)?.[0] as CFFECategory)?.name
                : capitalize(getCategories(props.modProps)?.[0] as string)
            }
          >
            <Tag
              img={
                isCurseForgeData(props.modProps.data) ? (
                  (getCategories(props.modProps)?.[0] as CFFECategory)?.iconUrl
                ) : (
                  <div>
                    <Show
                      fallback={getCategories(props.modProps)?.[0] as string}
                      when={props.modrinthCategories?.find(
                        (category) =>
                          category.name ===
                          (getCategories(props.modProps)?.[0] as string)
                      )}
                    >
                      <CategoryIcon
                        category={
                          props.modrinthCategories?.find(
                            (category) =>
                              category.name ===
                              (getCategories(props.modProps)?.[0] as string)
                          ) as MRFECategory
                        }
                      />
                    </Show>
                  </div>
                )
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
                            img={(tag as CFFECategory).iconUrl}
                            name={(tag as CFFECategory).name}
                            type="fixed"
                          />
                        )}
                      </For>
                    </Match>
                    <Match when={!isCurseForgeData(props.modProps.data)}>
                      <For each={getCategories(props.modProps).slice(1)}>
                        {(tag) => (
                          <Tag
                            img={
                              <div>
                                <Show
                                  when={props.modrinthCategories?.find(
                                    (category) => category.name === tag
                                  )}
                                >
                                  <CategoryIcon
                                    category={
                                      props.modrinthCategories?.find(
                                        (category) => category.name === tag
                                      ) as MRFECategory
                                    }
                                  />
                                </Show>
                              </div>
                            }
                            name={capitalize(tag as string)}
                            type="fixed"
                          />
                        )}
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
