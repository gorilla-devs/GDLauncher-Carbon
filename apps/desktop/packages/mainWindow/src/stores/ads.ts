import { createSignal } from "solid-js";

export interface BoundsSize {
  width: number;
  height: number;
  adSize: {
    width: number;
    height: number;
  };
}

export const loadMinimumBounds = async () => {
  const bounds = await window.getMinimumBounds();
  setMinimumBounds(bounds);
  window.minimumBoundsChanged((_, newBounds: BoundsSize) => {
    setMinimumBounds(newBounds);
  });
};

export const [minimumBounds, setMinimumBounds] = createSignal<BoundsSize>({
  width: 0,
  height: 0,
  adSize: {
    width: 0,
    height: 0,
  },
});
