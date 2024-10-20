import { LogEntry, LogEntryLevel } from "@/utils/logs";
import { port, rspc } from "@/utils/rspcClient";
import { Trans } from "@gd/i18n";
import { Outlet, useParams } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  createEffect,
  createSignal,
  onCleanup,
  onMount
} from "solid-js";
import { Button, Switch } from "@gd/ui";
import LogsSidebar from "./LogsSidebar";
import LogsContent from "./LogsContent";

export const [isFullScreen, setIsFullScreen] = createSignal(false);

const Logs = () => {
  const [logsCopied, setLogsCopied] = createSignal(false);
  const [logs, setLogs] = createSignal<LogEntry[]>([]);
  const params = useParams();

  const _logs = rspc.createQuery(() => ({
    queryKey: ["instance.getLogs", parseInt(params.id, 10)]
  }));

  const instanceLogs = () => {
    if (!_logs.data) {
      return undefined;
    }

    return _logs.data[_logs.data.length - 1];
  };

  createEffect(() => {
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

  // const copyLogsToClipboard = () => {
  //   window.copyToClipboard(JSON.stringify(instanceLogs()));
  //   setLogsCopied(true);
  // };

  // createEffect(() => {
  //   if (logsCopied()) {
  //     const timeoutId = setTimeout(() => {
  //       setLogsCopied(false);
  //     }, 400);

  //     onCleanup(() => {
  //       clearTimeout(timeoutId);
  //     });
  //   }
  // });

  // const [showButton, setShowButton] = createSignal(false);

  // const checkScrollTop = () => {
  //   const container = document.getElementById(
  //     "main-container-instance-details"
  //   );
  //   if (container) {
  //     if (!showButton() && container.scrollTop > 400) {
  //       setShowButton(true);
  //     } else if (showButton() && container.scrollTop <= 400) {
  //       setShowButton(false);
  //     }
  //   }
  // };

  // // Function to scroll to top smoothly
  // const scrollTop = () => {
  //   const container = document.getElementById(
  //     "main-container-instance-details"
  //   );
  //   if (container) {
  //     container.scrollTo({ top: 0, behavior: "smooth" });
  //   }
  // };

  // const container = document.getElementById("main-container-instance-details");
  // // Scroll event listener
  // onMount(() => {
  //   if (container) {
  //     container.addEventListener("scroll", checkScrollTop);
  //   }
  // });

  // onCleanup(() => {
  //   if (container) {
  //     container.removeEventListener("scroll", checkScrollTop);
  //   }
  // });

  createEffect(() => {
    if (isFullScreen()) {
      const container = document.getElementById("logs-content-box");

      if (container) {
        container.scrollIntoView({ block: "start", inline: "end" });
      }
    }
  });

  onCleanup(() => {
    setIsFullScreen(false);
  });

  return (
    <div class="h-full flex overflow-hidden" id="logs-content-box">
      <LogsSidebar logs={_logs.data || []} />
      <LogsContent />
    </div>
  );
};

export default Logs;
