import { createSignal, For, JSXElement, Show } from "solid-js";

interface TabType {
  name: string;
  component: JSXElement;
}

interface Props {
  tabs: TabType[];
  type?: "underline" | "block";
}

function Tabs(props: Props) {
  const [activeTab, setActiveTab] = createSignal<null | number>(null);
  //   const mergedProps = mergeProps({ type: "block" }, props);

  const handleClick = (index: number) => {
    if (activeTab() !== null && activeTab() === index) {
      setActiveTab(null);
    } else {
      setActiveTab(index);
    }
  };

  const Component = () => props.tabs[activeTab() || 0].component;

  return (
    <div class="flex flex-col">
      <div class="flex bg-black-black gap-6 border-b-black-semiblack border-b-1 box-border">
        <For each={props.tabs}>
          {(tab, i) => (
            <div
              class={`py-4 cursor-pointer relative ${
                activeTab() === i() ? "text-white" : "text-black-lightGray"
              }`}
              onClick={() => handleClick(i())}
            >
              {tab.name}
              <Show when={activeTab() === i()}>
                <div class="absolute left-0 right-0 bottom-0 h-1 bg-accent-main" />
              </Show>
            </div>
          )}
        </For>
      </div>
      <div>{Component()}</div>
    </div>
  );
}

export { Tabs };
