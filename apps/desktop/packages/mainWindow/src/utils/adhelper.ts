import { createStore } from "solid-js/store";

export interface BoundsSize {
  width: number;
  height: number;
  useVertical: boolean;
  useFallbackAd: boolean;
  shouldShow: boolean;
}

export const [adSize, _setAdSize] = createStore<BoundsSize>({
  width: 0,
  height: 0,
  useVertical: false,
  useFallbackAd: false,
  shouldShow: true
});

const init = async () => {
  const bounds = await window.getAdSize();
  _setAdSize(bounds);
  window.adSizeChanged((_, newBounds: Omit<BoundsSize, "shouldShow">) => {
    _setAdSize({
      ...newBounds,
      shouldShow: false
    });

    setTimeout(() => {
      _setAdSize({
        ...newBounds,
        shouldShow: true
      });
    }, 100);
  });
};

init();

export default adSize;
