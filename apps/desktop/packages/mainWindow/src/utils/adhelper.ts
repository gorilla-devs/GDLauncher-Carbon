import { createStore } from "solid-js/store";

export interface BoundsSize {
  width: number;
  height: number;
}

export const [adSize, _setAdSize] = createStore<BoundsSize>({
  width: 0,
  height: 0,
});

const init = async () => {
  const bounds = await window.getAdSize();
  _setAdSize(bounds);
  console.log("GIANMARCO NON SA FARE LE TABS", bounds);
  window.adSizeChanged((_, newBounds: BoundsSize) => {
    _setAdSize(newBounds);
  });
};

init();

export default adSize;
