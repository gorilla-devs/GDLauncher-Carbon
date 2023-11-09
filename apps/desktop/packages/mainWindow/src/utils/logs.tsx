import { createStore } from "solid-js/store";

export type Log = {
  data: string;
  type_: string;
};

export const [logsObj, setLogsObj] = createStore<{ [id: number]: Log[] }>({});
