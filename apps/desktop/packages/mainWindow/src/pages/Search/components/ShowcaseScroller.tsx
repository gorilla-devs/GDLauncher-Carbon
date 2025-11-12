import { useGDNavigate } from "@/managers/NavigationManager"
import { saveScrollPosition } from "@/utils/scrollRestoration"
import { FEUnifiedSearchResult } from "@gd/core_module/bindings"
import { Button, Skeleton } from "@gd/ui"
import { Trans } from "@gd/i18n"
import {
  For,
  JSX,
  Show,
  Suspense,
  createSignal,
  onCleanup,
  onMount
} from "solid-js"

interface ShowcaseScrollerProps {
  title: string
  elements: FEUnifiedSearchResult[]
  viewAllAction?: JSX.EventHandlerUnion<
    HTMLButtonElement,
    MouseEvent,
    JSX.EventHandler<HTMLButtonElement, MouseEvent>
  >
}

export default function ShowcaseScroller(props: ShowcaseScrollerProps) {
  const navigator = useGDNavigate()
  const [isDragging, setIsDragging] = createSignal(false)
  const [startX, setStartX] = createSignal(0)
  const [scrollLeft, setScrollLeft] = createSignal(0)
  const [dragStartTime, setDragStartTime] = createSignal(0)
  const [isMouseInside, setIsMouseInside] = createSignal(false)
  const [hasMouseMoved, setHasMouseMoved] = createSignal(false)
  let scrollContainer: HTMLDivElement | undefined

  const handleMouseMove = (e: MouseEvent, element: HTMLElement) => {
    const rect = element.getBoundingClientRect()
    const x = e.clientX - rect.left - rect.width / 2
    const y = e.clientY - rect.top - rect.height / 2
    const imageTransform = `perspective(300px) rotateY(${x / 10}deg) rotateX(${-y / 10}deg)`
    const textTransform = `perspective(300px) rotateY(${x / 8}deg) rotateX(${-y / 8}deg) translateZ(60px)`

    element.style.transform = "scale(1.1)"

    const imageElement = element.querySelector("img")!.parentElement
    if (imageElement) {
      imageElement.style.transform = imageTransform
    }
    const textElement = element.querySelector("h3") as HTMLElement
    if (textElement) {
      textElement.style.transform = textTransform
      textElement.style.textShadow = "6px 6px 12px rgba(0, 0, 0, 0.8)"
    }
  }

  const handleMouseLeave = (element: HTMLElement) => {
    element.style.transform = "scale(1)"

    const imageElement = element.querySelector("img")!.parentElement
    if (imageElement) {
      imageElement.style.transform =
        "perspective(300px) rotateY(0deg) rotateX(0deg)"
    }
    const textElement = element.querySelector("h3") as HTMLElement
    if (textElement) {
      textElement.style.transform =
        "perspective(300px) rotateY(0deg) rotateX(0deg) translateZ(0px)"
      textElement.style.textShadow = "none"
    }
  }

  const handleScroll = (e: WheelEvent) => {
    if (!scrollContainer || !isMouseInside() || !hasMouseMoved()) return

    const maxScroll = scrollContainer.scrollWidth - scrollContainer.clientWidth
    if (scrollContainer.scrollLeft < maxScroll && e.deltaY > 0) {
      e.preventDefault()
      scrollContainer.scrollLeft += e.deltaY
    } else if (scrollContainer.scrollLeft > 0 && e.deltaY < 0) {
      e.preventDefault()
      scrollContainer.scrollLeft += e.deltaY
    }
  }

  const handleDragStart = (e: MouseEvent, element: HTMLElement) => {
    if (!hasMouseMoved()) return
    setIsDragging(true)
    setStartX(e.pageX - element.offsetLeft)
    setScrollLeft(element.scrollLeft)
    setDragStartTime(Date.now())
  }

  const handleDragEnd = () => {
    setIsDragging(false)
  }

  const handleDragMove = (e: MouseEvent, element: HTMLElement) => {
    if (!isDragging()) return
    e.preventDefault()
    const x = e.pageX - element.offsetLeft
    const walk = (x - startX()) * 2
    element.scrollLeft = scrollLeft() - walk
  }

  const handleClick = (element: FEUnifiedSearchResult) => {
    // Only navigate if the drag duration was less than 150ms
    if (Date.now() - dragStartTime() < 150) {
      const contentWrapper = document.getElementById("gdl-content-wrapper")
      saveScrollPosition(contentWrapper)
      navigator.navigate(`/addon/${element.id}/${element.platform}`)
    }
  }

  onMount(() => {
    if (scrollContainer) {
      scrollContainer.addEventListener("wheel", handleScroll, {
        passive: false
      })

      onCleanup(() => {
        scrollContainer.removeEventListener("wheel", handleScroll)
      })
    }
  })

  return (
    <div class="z-0 flex flex-col gap-4 overflow-y-hidden">
      <div class="flex items-center justify-between">
        <h2 class="text-2xl font-bold">{props.title}</h2>
        <Show when={props.viewAllAction}>
          <Button type="text" onClick={props.viewAllAction}>
            <Trans key="general:_trn_common.view_all" />
          </Button>
        </Show>
      </div>
      <div class="relative">
        <div
          ref={scrollContainer}
          class="flex gap-4 overflow-x-auto overflow-y-hidden whitespace-nowrap px-6"
          onMouseDown={(e) =>
            handleDragStart(e, e.currentTarget as HTMLElement)
          }
          onMouseUp={handleDragEnd}
          onMouseLeave={() => {
            handleDragEnd()
            setIsMouseInside(false)
            setHasMouseMoved(false)
          }}
          onMouseEnter={() => setIsMouseInside(true)}
          onMouseMove={(e) => {
            if (isMouseInside() && !hasMouseMoved()) {
              setHasMouseMoved(true)
            }
            if (isDragging()) {
              handleDragMove(e, e.currentTarget as HTMLElement)
            }
          }}
          style={{
            cursor: isDragging() ? "grabbing" : "grab"
          }}
        >
          <Suspense
            fallback={
              <For each={new Array(15)}>
                {() => <Skeleton class="relative my-4 h-40 w-40 shrink-0" />}
              </For>
            }
          >
            <For each={props.elements}>
              {(element) => (
                <div
                  class="group relative my-4 w-40 shrink-0 transition-transform duration-300"
                  onMouseMove={(e) =>
                    !isDragging() &&
                    handleMouseMove(e, e.currentTarget as HTMLElement)
                  }
                  onMouseLeave={(e) =>
                    handleMouseLeave(e.currentTarget as HTMLElement)
                  }
                  onClick={() => handleClick(element)}
                >
                  <div
                    class="relative overflow-hidden"
                    style={{
                      transition: "transform 0.2s ease-out",
                      transform:
                        "perspective(300px) rotateY(0deg) rotateX(0deg)"
                    }}
                  >
                    <img
                      class="h-auto w-full rounded-lg object-cover transition duration-200 group-hover:scale-110"
                      src={element.imageUrl ?? ""}
                    />
                  </div>
                  <div class="absolute bottom-0 left-0 h-full w-full rounded-lg bg-black/50 transition-opacity duration-300 group-hover:opacity-0" />
                  <div class="absolute bottom-0 left-0 flex h-full w-full items-center justify-center transition-opacity duration-300">
                    <h3
                      class="truncate p-2 font-bold text-white"
                      style={{
                        transition: "all 0.2s ease-out",
                        transform:
                          "perspective(300px) rotateY(0deg) rotateX(0deg) translateZ(0px)"
                      }}
                    >
                      {element.title}
                    </h3>
                  </div>
                </div>
              )}
            </For>
          </Suspense>
          <Show when={props.viewAllAction}>
            <div
              class="w-42 bg-primary-400 my-4 flex shrink-0 cursor-pointer items-center justify-center rounded-lg"
              onClick={() => {
                navigator.navigate("/explore/list")
              }}
            >
              <h3 class="p-2 font-bold text-white">
                <Trans key="search:_trn_show_more" />
              </h3>
            </div>
          </Show>
        </div>
        {/* <div class="from-darkSlate-900 pointer-events-none absolute right-0 top-0 h-full w-24 bg-gradient-to-l to-transparent" /> */}
      </div>
    </div>
  )
}
