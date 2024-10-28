import { LogEntry } from "@/utils/logs";
import { port, rspc } from "@/utils/rspcClient";
import { useParams } from "@solidjs/router";
import { createEffect, createSignal, onCleanup, onMount } from "solid-js";
import LogsSidebar from "./LogsSidebar";
import LogsContent from "./LogsContent";
import { createStore } from "solid-js/store";

export const [isFullScreen, setIsFullScreen] = createSignal(false);

const Logs = () => {
  let logsContentRef: HTMLDivElement | undefined;
  let scrollBottomRef: HTMLDivElement | undefined;
  const [logs, setLogs] = createStore<LogEntry[]>([]);
  const [selectedLog, setSelectedLog] = createSignal<number | undefined>(
    undefined
  );
  const [autoFollowPreference, setAutoFollowPreference] = createSignal(true);
  const [autoFollow, setAutoFollow] = createSignal(true);
  const params = useParams();
  const [newLogsCount, setNewLogsCount] = createSignal(0);

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
      const newLogs = JSON.parse(event.data) as LogEntry[];

      setLogs((prev) => [...prev, ...newLogs]);

      if (!logsContentRef || !autoFollowPreference()) return;

      if (autoFollow()) {
        logsContentRef.scrollTop = logsContentRef.scrollHeight;
        setNewLogsCount(0);
      } else {
        setNewLogsCount((prev) => prev + 1);
      }
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

  createEffect(() => {
    // autoFollowPreference call NEEDS to be here for scrollToBottom to be called when it changes
    autoFollowPreference();
    selectedLog();
    setNewLogsCount(0);
    handleScroll();
  });

  const handleScroll = () => {
    if (!logsContentRef) return;

    const isAtBottom =
      logsContentRef.scrollHeight - logsContentRef.scrollTop ===
      logsContentRef.clientHeight;

    if (scrollBottomRef && (!autoFollowPreference() || !isActive())) {
      scrollBottomRef.style.display = "none";
      return;
    }

    if (isAtBottom) {
      setAutoFollow(true);
      if (scrollBottomRef && autoFollowPreference()) {
        scrollBottomRef.style.display = "none";
      }
    } else {
      setAutoFollow(false);
      if (scrollBottomRef && autoFollowPreference()) {
        scrollBottomRef.style.display = "flex";
      }
    }
  };

  onMount(() => {
    if (logsContentRef) {
      logsContentRef.addEventListener("scroll", handleScroll);
    }
  });

  onCleanup(() => {
    if (logsContentRef) {
      logsContentRef.removeEventListener("scroll", handleScroll);
    }
  });

  createEffect(() => {
    if (isFullScreen() && logsContentRef) {
      logsContentRef.scrollIntoView({
        block: "start",
        inline: "end"
      });
    }
  });

  onCleanup(() => {
    setIsFullScreen(false);
  });

  const scrollToBottom = () => {
    if (logsContentRef) {
      logsContentRef.scrollTop = logsContentRef.scrollHeight;
      setAutoFollow(true);
      setNewLogsCount(0);
      if (scrollBottomRef) {
        scrollBottomRef.style.display = "none";
      }
    }
  };

  function assignScrollBottomRef(ref: HTMLDivElement) {
    scrollBottomRef = ref;
  }

  function assignLogsContentRef(ref: HTMLDivElement) {
    logsContentRef = ref;
  }

  return (
    <div class="h-full w-full flex overflow-hidden border border-darkSlate-600 border-t-solid">
      <LogsSidebar
        availableLogEntries={availableLogEntries.data || []}
        setSelectedLog={setSelectedLog}
        selectedLog={selectedLog()}
        isLoading={availableLogEntries.isLoading}
      />
      <LogsContent
        logs={logs}
        isActive={isActive() || false}
        scrollToBottom={scrollToBottom}
        assignScrollBottomRef={assignScrollBottomRef}
        assignLogsContentRef={assignLogsContentRef}
        newLogsCount={newLogsCount()}
        autoFollowPreference={autoFollowPreference()}
        setAutoFollowPreference={setAutoFollowPreference}
      />
    </div>
  );
};

export default Logs;
