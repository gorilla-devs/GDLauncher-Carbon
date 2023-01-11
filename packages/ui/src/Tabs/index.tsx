import {
  createSignal,
  For,
  JSXElement,
  Match,
  mergeProps,
  Show,
  Switch,
} from "solid-js";

export interface TabType {
  name: string;
  component: JSXElement;
  icon?: string;
}

export interface Props {
  tabs: TabType[];
  type?: "underline" | "block";
}

function Tabs(props: Props) {
  const [activeTab, setActiveTab] = createSignal<number>(0);
  const mergedProps = mergeProps({ type: "underline" }, props);

  const handleClick = (index: number) => {
    setActiveTab(index);
  };

  const Component = () => props.tabs[activeTab() || 0].component;

  const UnderlinedTabs = () => {
    return (
      <div class="flex bg-black-black gap-6 border-b-black-semiblack border-b-1 box-border overflow-auto">
        <For each={props.tabs}>
          {(tab, i) => (
            <div
              class={`py-4 cursor-pointer min-w-fit relative font-500 capitalize ${
                activeTab() === i() ? "text-white" : "text-shade-0"
              }`}
              onClick={() => handleClick(i())}
            >
              {tab.name}
              <Show when={activeTab() === i()}>
                <div class="absolute left-0 right-0 bottom-0 h-1 bg-primary" />
              </Show>
            </div>
          )}
        </For>
      </div>
    );
  };

  const BlockTabs = () => {
    return (
      <div class="flex items-center bg-black-black p-1 rounded-xl h-10 box-border overflow-auto">
        <For each={props.tabs}>
          {(tab, i) => (
            <div
              class={`flex gap-1 justify-center items-center min-w-fit flex-1 py-2 h-full cursor-pointer rounded-xl font-500 capitalize box-border ${
                activeTab() === i()
                  ? "text-white bg-black-semiblack"
                  : "text-shade-0"
              }`}
              onClick={() => handleClick(i())}
            >
              <div class={`i-ri:${tab.icon}`} />
              {tab.name}
            </div>
          )}
        </For>
      </div>
    );
  };

  return (
    <div class="flex flex-col w-full">
      <Switch>
        <Match when={mergedProps.type === "underline"}>
          <UnderlinedTabs />
        </Match>
        <Match when={mergedProps.type === "block"}>
          <BlockTabs />
        </Match>
      </Switch>
      <div class="pt-5 max-w-full">{Component()}</div>
    </div>
  );
}

export { Tabs };
