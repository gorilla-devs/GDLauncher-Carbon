import { minimumBounds } from "@/stores/ads";
import { createEffect, onMount } from "solid-js";

export const AdsBanner = () => {
  createEffect(() => {
    console.log("adSizes", minimumBounds().adSize);
  });

  return (
    <div
      style={{
        height: `${minimumBounds().adSize.height}px`,
        width: `${minimumBounds().adSize.width}px`,
      }}
      class="bg-red-400 mx-5 mt-5"
    />
  );
};
