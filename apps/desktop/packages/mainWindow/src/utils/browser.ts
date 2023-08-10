import { createSignal } from "solid-js";

export const [scrollTop, setScrollTop] = createSignal<number>(0);
export const [instanceId, setInstanceId] = createSignal<undefined | number>(
  undefined
);
