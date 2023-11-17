import { onCleanup } from "solid-js";

export type RescrollerState = {
  /** The element that will be used to scroll */
  scrollRef?: HTMLElement;
  /** When set, is the position to scroll to on next mount */
  scrollTo?: number;
};

export type Rescroller = {
  /** Sets the element to scroll. */
  setScrollRefAndScrollIfNeeded: (ref: HTMLElement) => void;
  /** Queues scrolling for this component on next remount. */
  setScrollToFromCurrentPosition: () => void;
};

export function useRescroller(
  state: RescrollerState = {}
): [RescrollerState, Rescroller] {
  // Invalidate the scroll ref
  onCleanup(() => (state.scrollRef = undefined));

  return [
    state,
    {
      setScrollRefAndScrollIfNeeded: (ref) => {
        // Set the new ref
        state.scrollRef = ref;

        // If this component is due for a rescroll, rescroll
        state.scrollTo && state.scrollRef.scrollTo(0, state.scrollTo);

        // Clear the scroll, as that's what we just did
        state.scrollTo = undefined;
      },
      setScrollToFromCurrentPosition: () =>
        state.scrollRef && (state.scrollTo = state.scrollRef.scrollTop)
    }
  ];
}
