import { createSignal } from "solid-js";

export interface BoundsSize {
  width: number;
  height: number;
  AdSize: {
    width: number;
    height: number;
  };
}

export const [minimumBounds, setMinimumBounds] = createSignal<BoundsSize>({
  width: 0,
  height: 0,
  AdSize: {
    width: 0,
    height: 0,
  },
});
