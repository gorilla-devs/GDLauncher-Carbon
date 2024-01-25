import { JSX, Show, children } from "solid-js";

type Props = {
  children: JSX.Element;
  imageUrl?: string;
};

const FadedBanner = (props: Props) => {
  let c = children(() => props.children);

  return (
    <>
      <div class="absolute z-10 bg-gradient-to-r from-darkSlate-700 from-50% inset-0" />
      <div class="absolute right-0 from-darkSlate-700 z-10 bg-gradient-to-r top-0 bottom-0 w-1/2" />
      <Show when={props.imageUrl}>
        <img
          class="absolute right-0 top-0 bottom-0 select-none w-1/2 z-0"
          src={props.imageUrl}
        />
      </Show>
      {c()}
    </>
  );
};

export const FadedBannerSkeleton = () => {
  return (
    <div class="w-full h-full">
      <div class="w-full h-full bg-gray-700" />
    </div>
  );
};

export default FadedBanner;
