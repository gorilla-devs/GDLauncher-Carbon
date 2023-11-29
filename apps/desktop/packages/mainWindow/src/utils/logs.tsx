import { createStore } from "solid-js/store";

export type LogEntry = {
  sourceKind: LogEntrySourceKind;
  level: LogEntryLevel;
  timestamp: number;
  logger: string;
  thread: string;
  message: string;
};

export enum LogEntrySourceKind {
  _System = "System",
  _StdOut = "StdOut",
  _StdErr = "StdErr"
}

// eslint-disable-next-line no-unused-vars
export enum LogEntryLevel {
  // eslint-disable-next-line no-unused-vars
  Trace = "Trace",
  // eslint-disable-next-line no-unused-vars
  Debug = "Debug",
  // eslint-disable-next-line no-unused-vars
  Info = "Info",
  // eslint-disable-next-line no-unused-vars
  Warn = "Warn",
  // eslint-disable-next-line no-unused-vars
  Error = "Error"
}

export const [logsObj, setLogsObj] = createStore<{ [id: number]: LogEntry[] }>(
  {}
);
