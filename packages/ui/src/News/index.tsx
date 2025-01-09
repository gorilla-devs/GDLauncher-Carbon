import {
  createEffect,
  createSignal,
  For,
  mergeProps,
  onCleanup,
  onMount,
  Show
} from "solid-js"

interface SlideProps {
  image: string
  title: string
  description: string
  url?: string
}

interface SliderProps {
  currentImageIndex: number
  slides: SlideProps[]
  alignment?: string
  onClick?: (_news: SlideProps) => void
  fallBackImg: string | undefined
  onSlideClick?: (_news: SlideProps) => void
}

interface CarouselProps {
  slides: SlideProps[]
  speed?: number
  rtl?: boolean
  disableAutoRotation?: boolean
  showArrows?: boolean
  showIndicators?: boolean
  fallBackImg?: string
  onClick?: (_news: SlideProps) => void
}

const News = (props: CarouselProps) => {
  const [currentImageIndex, setCurrentImageIndex] = createSignal(1)
  const [isMoving, setIsMoving] = createSignal(false)
  let interval: ReturnType<typeof setInterval>

  const mergedProps = mergeProps(
    { showIndicators: true, showArrows: true, rtl: true },
    props
  )

  let slidesRef: HTMLDivElement

  const moveSlide = () => {
    slidesRef.style.transform = `translateX(-${currentImageIndex() * 100}%)`
  }

  const firstSlide = () => props.slides[0]
  const lastSlide = () => props.slides[props.slides.length - 1]
  const copiedSlides = () => [lastSlide(), ...props.slides, firstSlide()]

  const resetInterval = () => {
    clearInterval(interval)
    interval = setInterval(() => {
      if (!props.disableAutoRotation) {
        changeSlide(mergedProps.rtl ? "right" : "left")
      }
    }, props.speed || 5000)
  }

  const handleTransitionEnd = () => {
    setIsMoving(false)
    if (currentImageIndex() === 0) {
      slidesRef.style.transition = `none`
      setCurrentImageIndex(copiedSlides().length - 2)
      moveSlide()
    }
    if (currentImageIndex() === copiedSlides().length - 1) {
      slidesRef.style.transition = `none`
      setCurrentImageIndex(1)
      moveSlide()
    }
  }

  onMount(() => {
    slidesRef.addEventListener("transitionend", handleTransitionEnd)
  })

  onCleanup(() => {
    slidesRef.removeEventListener("transitionend", handleTransitionEnd)
  })

  const changeSlide = (direction: "right" | "left") => {
    resetInterval()
    setIsMoving(true)
    slidesRef.style.transition = `transform 450ms ease-in-out`
    if (currentImageIndex() > props.slides.length) return
    if (direction === "right") {
      setCurrentImageIndex((prev) => prev + 1)
    } else {
      setCurrentImageIndex((prev) => prev - 1)
    }
    moveSlide()
  }

  createEffect(() => {
    resetInterval()
  })

  onCleanup(() => clearInterval(interval))

  const Slider = (props: SliderProps) => {
    onMount(() => {
      moveSlide()
    })

    return (
      <div ref={slidesRef} class="flex h-24">
        <For each={copiedSlides()}>
          {(slide) => (
            <div
              class="inset-0 box-border flex h-24 w-full flex-shrink-0 flex-grow transform items-center justify-center bg-cover bg-center bg-no-repeat"
              style={{
                "background-image": `url('${slide.image}'), url('${props.fallBackImg}')`
              }}
            >
              <div
                class="absolute bottom-0 left-0 right-0 top-0"
                style={{
                  background: "rgba(29, 32, 40, 0.7)"
                }}
              />
              <div class="absolute box-border flex h-full w-full items-center px-4 pb-6">
                <div class="flex select-none flex-col gap-1">
                  <div
                    class="group flex cursor-pointer items-center gap-2"
                    onClick={() => props.onSlideClick?.(slide)}
                  >
                    <h2 class="m-0 overflow-hidden text-ellipsis whitespace-nowrap group-hover:underline">
                      {slide.title}
                    </h2>
                    <div class="i-ri:external-link-line peer" />
                  </div>
                  <h4
                    class="text-lightSlate-400 m-0 overflow-hidden text-ellipsis whitespace-nowrap
                "
                  >
                    {slide.description}
                  </h4>
                </div>
              </div>
            </div>
          )}
        </For>
      </div>
    )
  }

  return (
    <div class="bg-darkSlate-900 group relative h-24 overflow-hidden rounded-lg">
      <Show when={mergedProps.showArrows}>
        <div
          class="bg-darkSlate-800 absolute left-5 top-1/2 z-40 flex h-7 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full opacity-0 duration-200 ease-in-out group-hover:opacity-100 "
          onClick={() => {
            if (isMoving()) return
            changeSlide("left")
          }}
        >
          <div class="i-ri:arrow-drop-left-line text-lightSlate-50 text-3xl" />
        </div>
        <div
          class="bg-darkSlate-800 absolute right-5 top-1/2 z-40 flex h-7 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full opacity-0 duration-200 ease-in-out group-hover:opacity-100"
          onClick={() => {
            if (isMoving()) return
            changeSlide("right")
          }}
        >
          <div class="i-ri:arrow-drop-right-line text-lightSlate-50 text-3xl" />
        </div>
      </Show>
      <Show when={mergedProps.showIndicators}>
        <div class="absolute bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center justify-between gap-2">
          <For each={copiedSlides().slice(1, -1)}>
            {(_, i) => (
              <div
                class={`h-2 w-2 cursor-pointer rounded-full bg-white ${
                  currentImageIndex() === i() + 1 ? "opacity-100" : "opacity-30"
                }`}
                onClick={() => {
                  resetInterval()
                  slidesRef.style.transition = `transform 450ms ease-in-out`
                  setCurrentImageIndex(i() + 1)
                  moveSlide()
                }}
              />
            )}
          </For>
        </div>
      </Show>
      <Show when={props.slides}>
        <Slider
          fallBackImg={props.fallBackImg}
          currentImageIndex={currentImageIndex()}
          slides={props.slides}
          onSlideClick={(news) => props?.onClick?.(news)}
        />
      </Show>
    </div>
  )
}

export { News }
