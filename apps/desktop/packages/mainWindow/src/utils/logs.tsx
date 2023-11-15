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
  System = "System",
  StdOut = "StdOut",
  StdErr = "StdErr"
}

export enum LogEntryLevel {
  Trace = "Trace",
  Debug = "Debug",
  Info = "Info",
  Warn = "Warn",
  Error = "Error"
}

export const [logsObj, setLogsObj] = createStore<{ [id: number]: LogEntry[] }>(
  {}
);
