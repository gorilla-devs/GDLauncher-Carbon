import { createStore } from "solid-js/store";

export interface BoundsSize {
  width: number;
  height: number;
  adSize: {
    width: number;
    height: number;
    padding: number;
  };
}

export const [minimumBounds, setMinimumBounds] = createStore<BoundsSize>({
  width: 0,
  height: 0,
  adSize: {
    width: 0,
    height: 0,
    padding: 20
  },
});

export const init = async () => {
  const bounds = await window.getMinimumBounds();
  setMinimumBounds(bounds);
  window.minimumBoundsChanged((_, newBounds: BoundsSize) => {
    setMinimumBounds(newBounds);
  });
};

export default minimumBounds;