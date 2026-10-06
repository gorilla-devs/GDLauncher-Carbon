import {
  createSignal,
  For,
  mergeProps,
  onCleanup,
  onMount,
  Show
} from "solid-js"
import { cn } from "../util"

interface SlideProps {
  image: string
  title: string
  description: string
  url?: string
  date: string
}

interface SliderProps {
  currentImageIndex: number
  slides: SlideProps[]
  alignment?: string
  onClick?: (_news: SlideProps) => void
  fallBackImg: string | undefined
  onSlideClick?: (_news: SlideProps) => void
  className?: string
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
  className?: string
}

const News = (props: CarouselProps) => {
  const [currentImageIndex, setCurrentImageIndex] = createSignal(1)
  const [isMoving, setIsMoving] = createSignal(false)

  const mergedProps = mergeProps(
    { showIndicators: true, showArrows: true, rtl: true },
    props
  )

  let slidesRef!: HTMLDivElement

  const moveSlide = () => {
    slidesRef.style.transform = `translateX(-${currentImageIndex() * 100}%)`
  }

  const firstSlide = () => props.slides[0]
  const lastSlide = () => props.slides[props.slides.length - 1]
  const copiedSlides = () => [lastSlide(), ...props.slides, firstSlide()]

  let intervalId: number | undefined

  const resetInterval = () => {
    if (intervalId !== undefined) {
      clearInterval(intervalId)
    }
    intervalId = setInterval(() => {
      if (!props.disableAutoRotation) {
        changeSlide(mergedProps.rtl ? "right" : "left")
      }
    }, props.speed || 5000) as unknown as number
  }

  let movementGeneration = 0
  let completionTimer: ReturnType<typeof setTimeout> | undefined
  let removeCompletionListeners: (() => void) | undefined

  const retireMovement = () => {
    movementGeneration++
    clearTimeout(completionTimer)
    completionTimer = undefined
    removeCompletionListeners?.()
    removeCompletionListeners = undefined
  }

  const completeMovement = (generation: number) => {
    if (generation !== movementGeneration || !slidesRef) return
    retireMovement()
    if (currentImageIndex() === 0) {
      slidesRef.style.transition = "none"
      setCurrentImageIndex(props.slides.length)
      moveSlide()
    } else if (currentImageIndex() === copiedSlides().length - 1) {
      slidesRef.style.transition = "none"
      setCurrentImageIndex(1)
      moveSlide()
    }
    setIsMoving(false)
  }

  const transitionTime = () => {
    if (slidesRef.closest(".potato-pc")) return 0
    const style = getComputedStyle(slidesRef)
    const properties = style.transitionProperty
      .split(",")
      .map((value) => value.trim())
    const milliseconds = (value: string) => {
      const n = Number.parseFloat(value)
      return Number.isFinite(n)
        ? n * (value.trim().endsWith("ms") ? 1 : 1000)
        : 0
    }
    const durations = style.transitionDuration.split(",").map(milliseconds)
    const delays = style.transitionDelay.split(",").map(milliseconds)
    return properties.reduce((max, property, i) => {
      if (property !== "transform" && property !== "all") return max
      const duration = durations[i % durations.length]
      return duration > 0
        ? Math.max(max, duration + delays[i % delays.length])
        : max
    }, 0)
  }

  const selectSlide = (index: number) => {
    if (
      !slidesRef ||
      !props.slides.length ||
      index < 0 ||
      index > props.slides.length + 1
    )
      return
    retireMovement()
    const generation = movementGeneration
    const startedAt = new Event("transitionstart").timeStamp
    const startedClock = performance.now()
    resetInterval()
    if (index === currentImageIndex()) {
      completeMovement(generation)
      return
    }
    setIsMoving(true)
    slidesRef.style.transition = "transform 450ms ease"
    setCurrentImageIndex(index)
    moveSlide()
    const time = transitionTime()
    if (time <= 0) {
      completeMovement(generation)
      return
    }
    const complete = (event: TransitionEvent) => {
      if (
        event.target !== slidesRef ||
        event.propertyName !== "transform" ||
        event.timeStamp < startedAt
      )
        return
      // A canceled previous transform may dispatch after its replacement
      // starts. It cannot release a replacement that still has motion time.
      if (performance.now() - startedClock < time && transitionTime() > 0)
        return
      completeMovement(generation)
    }
    slidesRef.addEventListener("transitionend", complete)
    slidesRef.addEventListener("transitioncancel", complete)
    removeCompletionListeners = () => {
      slidesRef.removeEventListener("transitionend", complete)
      slidesRef.removeEventListener("transitioncancel", complete)
    }
    completionTimer = setTimeout(
      () => completeMovement(generation),
      Math.min(time + 50, 5000)
    )
  }

  onMount(resetInterval)
  onCleanup(() => {
    retireMovement()
    if (intervalId !== undefined) clearInterval(intervalId)
  })

  const changeSlide = (direction: "right" | "left") => {
    selectSlide(currentImageIndex() + (direction === "right" ? 1 : -1))
  }

  const Slider = (props: SliderProps) => {
    onMount(() => {
      moveSlide()
    })

    return (
      <div ref={slidesRef} class="flex h-full">
        <For each={copiedSlides()}>
          {(slide) => (
            <div
              class="inset-0 box-border flex h-full w-full flex-shrink-0 flex-grow transform items-center justify-center bg-cover bg-center bg-no-repeat"
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
              <div
                class="absolute box-border flex h-full w-full p-8 cursor-pointer group"
                onClick={() => props.onSlideClick?.(slide)}
              >
                <div class="flex select-none flex-col gap-2 w-full">
                  <div class="flex items-center gap-2 text-3xl">
                    <h1 class="m-0 overflow-hidden text-ellipsis whitespace-nowrap group-hover:underline">
                      {slide.title}
                    </h1>
                  </div>
                  <h2 class="text-lightSlate-200 m-0 w-full overflow-hidden text-ellipsis whitespace-nowrap text-xl">
                    {slide.description}
                  </h2>
                </div>

                <div class="absolute bottom-0 left-0 p-8">
                  <p class="text-lightSlate-200 text-sm">
                    {new Date(slide.date).toLocaleDateString(undefined, {
                      year: "numeric",
                      month: "short",
                      day: "numeric"
                    })}
                  </p>
                </div>
              </div>
            </div>
          )}
        </For>
      </div>
    )
  }

  return (
    <div
      class={cn(
        "bg-darkSlate-900 group relative h-84 overflow-hidden rounded-lg",
        props.className
      )}
    >
      <Show when={mergedProps.showArrows}>
        <div
          class="bg-darkSlate-800 absolute left-5 top-1/2 z-40 flex h-7 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full opacity-0 duration-200 ease-spring group-hover:opacity-100 "
          onClick={() => {
            if (isMoving()) return
            changeSlide("left")
          }}
        >
          <div class="i-hugeicons:arrow-left-01 text-3xl text-lightSlate-50" />
        </div>
        <div
          class="bg-darkSlate-800 absolute right-5 top-1/2 z-40 flex h-7 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full opacity-0 duration-200 ease-spring group-hover:opacity-100"
          onClick={() => {
            if (isMoving()) return
            changeSlide("right")
          }}
        >
          <div class="i-hugeicons:arrow-right-01 text-3xl text-lightSlate-50" />
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
                  selectSlide(i() + 1)
                }}
              />
            )}
          </For>
        </div>
      </Show>
      <Show when={props.slides.length > 0}>
        <Slider
          fallBackImg={props.fallBackImg}
          currentImageIndex={currentImageIndex()}
          slides={props.slides}
          onSlideClick={(news) => props?.onClick?.(news)}
          className={props.className}
        />
      </Show>
    </div>
  )
}

export { News }
