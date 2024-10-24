import { LogEntry } from "@/utils/logs";
import { port, rspc } from "@/utils/rspcClient";
import { useParams } from "@solidjs/router";
import { createEffect, createSignal, onCleanup } from "solid-js";
import LogsSidebar from "./LogsSidebar";
import LogsContent from "./LogsContent";
import { createStore } from "solid-js/store";

export const [isFullScreen, setIsFullScreen] = createSignal(false);

const Logs = () => {
  let logsContainerRef: HTMLDivElement | undefined;
  // const [logsCopied, setLogsCopied] = createSignal(false);
  const [logs, setLogs] = createStore<LogEntry[]>([]);
  const [selectedLog, setSelectedLog] = createSignal<number | undefined>(
    undefined
  );
  const params = useParams();

  const availableLogEntries = rspc.createQuery(() => ({
    queryKey: ["instance.getLogs", parseInt(params.id, 10)]
  }));

  const isActive = () =>
    availableLogEntries.data?.find((log) => log.id === selectedLog())?.active;

  createEffect(() => {
    if (!availableLogEntries.data) return;
    const activeLogId = availableLogEntries.data.find((log) => log.active)?.id;

    if (activeLogId !== undefined) setSelectedLog(activeLogId);
  });

  createEffect(() => {
    if (selectedLog() === undefined) return;

    const wsConnection = new WebSocket(
      `ws://127.0.0.1:${port}/instance/log?id=${selectedLog()}`
    );

    wsConnection.onmessage = (event) => {
      const newLog = JSON.parse(event.data) as LogEntry;
      setLogs(logs.length, newLog);
    };

    onCleanup(() => {
      setLogs([]);

      if (wsConnection && wsConnection.readyState === wsConnection.OPEN) {
        wsConnection.close();
      }
    });
  });

  onCleanup(() => {
    setSelectedLog(undefined);
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
    if (isFullScreen() && logsContainerRef) {
      logsContainerRef.scrollIntoView({
        block: "start",
        inline: "end"
      });
    }
  });

  onCleanup(() => {
    setIsFullScreen(false);
  });

  return (
    <div
      class="h-full w-full flex overflow-hidden border border-darkSlate-600 border-t-solid"
      ref={(ref) => (logsContainerRef = ref)}
    >
      <LogsSidebar
        availableLogEntries={availableLogEntries.data || []}
        setSelectedLog={setSelectedLog}
        selectedLog={selectedLog()}
      />
      <LogsContent logs={logs} isActive={isActive() || false} />
    </div>
  );
};

export default Logs;
