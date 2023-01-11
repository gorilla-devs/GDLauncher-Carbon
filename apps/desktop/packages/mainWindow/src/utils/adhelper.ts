import { createStore } from "solid-js/store";

export interface BoundsSize {
  width: number;
  height: number;
}

export const [adSize, _setMinimumBounds] = createStore<BoundsSize>({
  width: 0,
  height: 0,
});

export const init = async () => {
  const bounds = await window.getMinimumBounds();
  _setMinimumBounds(bounds);
  window.minimumBoundsChanged((_, newBounds: BoundsSize) => {
    _setMinimumBounds(newBounds);
  });
};

export default adSize;
