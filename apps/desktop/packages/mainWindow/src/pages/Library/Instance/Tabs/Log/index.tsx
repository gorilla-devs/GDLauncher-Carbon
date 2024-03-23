import { LogEntry, LogEntryLevel } from "@/utils/logs";
import { port } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { useRouteData } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  onMount
} from "solid-js";
import fetchData from "../../instance.logs.data";
import { Button, Tooltip } from "@gd/ui";

const Logs = () => {
  const [logsCopied, setLogsCopied] = createSignal(false);
  const [logs, setLogs] = createSignal<LogEntry[]>([]);

  const routeData = useRouteData<typeof fetchData>();

  const instanceLogs = () => {
    if (!routeData.logs.data) {
      return undefined;
    }

    return routeData.logs.data[routeData.logs.data.length - 1];
  };

  createEffect(async () => {
    if (instanceLogs()) {
      setLogs([]);

      const wsConnection = new WebSocket(
        `ws://127.0.0.1:${port}/instance/log?id=${instanceLogs()?.id}`
      );

      wsConnection.onmessage = (event) => {
        const newLog = JSON.parse(event.data) as LogEntry;
        setLogs((prevLogs) => [...prevLogs, newLog]);
      };

      onCleanup(() => {
        if (wsConnection && wsConnection.readyState === wsConnection.OPEN) {
          wsConnection.close();
        }
      });
    }
  });

  const copyLogsToClipboard = () => {
    window.copyToClipboard(JSON.stringify(instanceLogs()));
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
      <div class="pb-4 max-h-full flex flex-col divide-y divide-darkSlate-500 divide-x-none divide-solid select-text">
        <Switch>
          <Match when={(logs().length || 0) > 0}>
            <For each={logs()}>
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
          <Match when={(logs().length || 0) === 0}>
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
