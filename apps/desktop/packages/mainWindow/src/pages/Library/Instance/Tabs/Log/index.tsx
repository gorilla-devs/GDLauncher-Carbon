import { LogEntryLevel, logsObj, setLogsObj } from "@/utils/logs";
import { port } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useParams, useRouteData } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createResource,
  createSignal,
  onCleanup,
  onMount
} from "solid-js";
import fetchData from "../../instance.logs.data";
import { getRunningState } from "@/utils/instances";
import { Button, Tooltip } from "@gd/ui";

const fetchLogs = async (logId: number) => {
  return fetch(`http://localhost:${port}/instance/log?id=${logId}`);
};

const Logs = () => {
  const [logsCopied, setLogsCopied] = createSignal(false);
  const params = useParams();

  const routeData = useRouteData<typeof fetchData>();

  const instanceId = () => parseInt(params.id, 10);

  const instanceLogs = () =>
    routeData.logs.data
      ?.reverse()
      .find((item) => item.instance_id === instanceId());

  const logId = () => instanceLogs()?.id;

  const [allLogs, { refetch }] = createResource(logId, fetchLogs);

  async function streamToJson(readableStream: ReadableStream) {
    // Get the reader from the stream
    // eslint-disable-next-line no-undef
    let reader: ReadableStreamDefaultReader;
    // Check if the stream is locked
    if (readableStream.locked) {
      // Stream is locked, use the existing reader
      if (readableStream.locked) {
        throw new Error("ReadableStream is locked but no reader found");
      }
      reader = readableStream.getReader();
    } else {
      // Stream is not locked, get a new reader
      reader = readableStream.getReader();
    }

    // Read the stream
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const { value, done } = await reader.read();

      if (done) {
        // Stream has ended
        break;
      }

      const fixedJson =
        "[" +
        new TextDecoder("utf-8").decode(value).replace(/}{/g, "},{") +
        "]";
      const json = JSON.parse(fixedJson);
      setLogsObj(instanceId(), (prev) => [...(prev || []), ...json]);
    }
  }

  createEffect(() => {
    if (routeData.instanceDetails.data) {
      const isRunning = getRunningState(routeData.instanceDetails.data.state);
      if (isRunning) {
        refetch();
      }
    }
  });

  createEffect(() => {
    if (allLogs()?.body) {
      streamToJson((allLogs() as any).body);
    }
  });

  const instanceLogss = () => logsObj[instanceId()] || [];

  const copyLogsToClipboard = () => {
    window.copyToClipboard(JSON.stringify(instanceLogss()));
    setLogsCopied(true);
  };

  createEffect(() => {
    if (logsCopied()) {
      const timeoutId = setTimeout(() => {
        setLogsCopied(false);
      }, 400);

      onCleanup(() => {
        clearTimeout(timeoutId);
      });
    }
  });

  const [showButton, setShowButton] = createSignal(false);

  const checkScrollTop = () => {
    const container = document.getElementById(
      "main-container-instance-details"
    );
    if (container) {
      if (!showButton() && container.scrollTop > 400) {
        setShowButton(true);
      } else if (showButton() && container.scrollTop <= 400) {
        setShowButton(false);
      }
    }
  };

  // Function to scroll to top smoothly
  const scrollTop = () => {
    const container = document.getElementById(
      "main-container-instance-details"
    );
    if (container) {
      container.scrollTo({ top: 0, behavior: "smooth" });
    }
  };

  const container = document.getElementById("main-container-instance-details");
  // Scroll event listener
  onMount(() => {
    if (container) {
      container.addEventListener("scroll", checkScrollTop);
    }
  });

  onCleanup(() => {
    if (container) {
      container.removeEventListener("scroll", checkScrollTop);
    }
  });

  return (
    <div>
      <Show when={showButton()}>
        <div class="rounded-full fixed bottom-4 right-[490px]">
          <Button typeof="secondary" onClick={scrollTop}>
            <Trans key="logs.scroll_top" />
          </Button>
        </div>
      </Show>
      <div class="w-full flex justify-end px-4 py-2 box-border">
        <Tooltip content={logsCopied() ? "Copied" : "Copy"}>
          <Button type="secondary" onClick={copyLogsToClipboard}>
            <div class="i-ri:file-copy-fill cursor-pointer" />
            <span>
              <Trans key="logs.copy" />
            </span>
          </Button>
        </Tooltip>
      </div>
      <div class="pb-4 max-h-full flex flex-col divide-y divide-x-none divide-solid divide-darkSlate-500 select-text">
        <Switch>
          <Match when={(instanceLogss().length || 0) > 0}>
            <For each={instanceLogss()}>
              {(log) => {
                let levelColorClass = "";

                switch (log.level) {
                  case LogEntryLevel.Trace: {
                    levelColorClass = "text-gray-500";

                    break;
                  }
                  case LogEntryLevel.Debug: {
                    levelColorClass = "text-orange-500";

                    break;
                  }
                  case LogEntryLevel.Info: {
                    levelColorClass = "text-green-500";

                    break;
                  }
                  case LogEntryLevel.Warn: {
                    levelColorClass = "text-text-500";

                    break;
                  }
                  case LogEntryLevel.Error: {
                    levelColorClass = "text-red-500";

                    break;
                  }
                }

                return (
                  <div class="flex flex-col justify-center items-center w-full overflow-x-auto scrollbar-hide">
                    <pre class="m-0 w-full box-border leading-8">
                      <code class="text-darkSlate-50 text-sm select-text">
                        <span class={levelColorClass}>
                          [{log.level.toUpperCase()}]
                        </span>{" "}
                        {log.logger}@{log.thread}
                        {": "}
                        {log?.message}
                      </code>
                    </pre>
                  </div>
                );
              }}
            </For>
          </Match>
          <Match when={(instanceLogss().length || 0) === 0}>
            <div class="h-full flex justify-center items-center">
              <p>
                <Trans key="logs.no_logs" />
              </p>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

export default Logs;
