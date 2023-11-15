import { createStore } from "solid-js/store";

export type LogEntry = {
  sourceKind: string;
  level: string;
  timestamp: number;
  logger: string;
  thread: string;
  message: string;
};

export const [logsObj, setLogsObj] = createStore<{ [id: number]: LogEntry[] }>(
  {}
);
