import { JSX, Show, children } from "solid-js"

interface Props {
  children: JSX.Element
  imageUrl?: string
}

const FadedBanner = (props: Props) => {
  const c = children(() => props.children)

  return (
    <>
      <div class="from-darkSlate-700 absolute inset-0 z-10 bg-gradient-to-r from-50%" />
      <div class="from-darkSlate-700 absolute bottom-0 right-0 top-0 z-10 w-1/2 bg-gradient-to-r" />
      <Show when={props.imageUrl}>
        <img
          class="absolute bottom-0 right-0 top-0 z-0 w-1/2 select-none"
          src={props.imageUrl}
        />
      </Show>
      {c()}
    </>
  )
}

export const FadedBannerSkeleton = () => {
  return (
    <div class="h-full w-full">
      <div class="h-full w-full bg-gray-700" />
    </div>
  )
}

export default FadedBanner
