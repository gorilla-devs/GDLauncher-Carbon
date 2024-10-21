import { Input } from "@gd/ui";
import { For } from "solid-js";
import { isFullScreen, setIsFullScreen } from ".";

const LogsContent = () => {
  return (
    <div class="flex-1 flex flex-col border border-darkSlate-700 border-l-solid">
      <div class="flex justify-between items-center gap-4 w-full h-10 bg-darkSlate-800 py-8 px-4 box-border">
        <Input icon={<div class="i-ri:search-line" />} placeholder="Search" />
        <div
          class="w-6 h-6 bg-lightSlate-800 hover:bg-lightSlate-50 transition-colors duration-200 ease-in-out"
          classList={{
            "i-ri:fullscreen-line": !isFullScreen(),
            "i-ri:fullscreen-exit-line": isFullScreen()
          }}
          onClick={() => {
            setIsFullScreen(!isFullScreen());
          }}
        />
      </div>
      <div
        class="bg-darkSlate-900 flex-1 overflow-y-scroll px-4 py-2"
        id="instance_logs_container"
      >
        <For each={new Array(100).fill(0)}>
          {(_, index) => <div>Hello world this is a line</div>}
        </For>
        <div>This is the last line</div>
      </div>
    </div>
  );
};

export default LogsContent;
