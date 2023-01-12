import { Match, Switch, createEffect } from "solid-js";
import { useTabsContext } from "./Tabs";

interface Props {
  children: Element[];
}

const TabList = (props: Props) => {
  const tabsContext = useTabsContext();

  // const mergedProps = mergeProps({ type: "underline" }, props);

  const UnderlinedTabs = () => {
    return (
      <div class="flex bg-black-black gap-6 border-b-black-semiblack border-b-1 box-border overflow-auto">
        {/* <For each={props.tabs}>
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
        </For> */}
      </div>
    );
  };

  // const BlockTabs = () => {
  //   return (
  //     <div class="flex items-center bg-black-black p-1 rounded-xl h-10 box-border overflow-auto">
  //       <For each={props.tabs}>
  //         {(tab, i) => (
  //           <div
  //             class={`flex gap-1 justify-center items-center min-w-fit flex-1 py-2 h-full cursor-pointer rounded-xl font-500 capitalize box-border ${
  //               activeTab() === i()
  //                 ? "text-white bg-black-semiblack"
  //                 : "text-shade-0"
  //             }`}
  //             onClick={() => handleClick(i())}
  //           >
  //             <div class={`i-ri:${tab.icon}`} />
  //             {tab.name}
  //           </div>
  //         )}
  //       </For>
  //     </div>
  //   );
  // };

  createEffect(() => {
    console.log("CHANGe", tabsContext?.type);
  });

  return (
    <div class="flex flex-col w-full h-auto">
      <Switch>
        <Match when={tabsContext?.type === "underline"}>
          {/* <div class="flex bg-black-black gap-6 border-b-black-semiblack border-b-1 box-border overflow-auto"> */}
          <div
            class="flex gap-6 border-b-shade-8 border-b-1 box-border overflow-auto"
            classList={{
              row: tabsContext?.orientation === "horizontal",
              col: tabsContext?.orientation === "vertical",
            }}
          >
            {props.children}
          </div>
        </Match>
        <Match when={tabsContext?.type === "block"}>
          <div
            class="flex items-center p-1 rounded-xl h-10 box-border overflow-auto"
            classList={{
              row: tabsContext?.orientation === "horizontal",
              col: tabsContext?.orientation === "vertical",
            }}
          >
            {props.children}
          </div>
        </Match>
      </Switch>
    </div>
  );
  //   return <div>{props.children}</div>;
};

export default TabList;
