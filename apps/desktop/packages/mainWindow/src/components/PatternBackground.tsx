import { createEffect, onCleanup } from "solid-js"
import { render } from "solid-js/web"
import InlinePatternSVG from "./InlinePatternSVG"

interface Props {
  children: any
  tileSize?: number
  animationDuration?: number
}

interface Tile {
  element: HTMLDivElement
  x: number
  y: number
  distance?: number
}

const PatternBackground = (props: Props) => {
  const tileSize = props.tileSize || 100
  const duration = props.animationDuration || 1200 // ms
  let containerRef: HTMLDivElement | undefined

  createEffect(() => {
    if (!containerRef) return

    const container = document.createElement("div")
    container.className = "pattern-container absolute inset-0"
    containerRef.appendChild(container)

    const cols = Math.ceil(window.innerWidth / tileSize)
    const rows = Math.ceil(window.innerHeight / tileSize)
    const tiles: Tile[] = []

    for (let y = 0; y < rows; y++) {
      for (let x = 0; x < cols; x++) {
        const tile = document.createElement("div")
        tile.className = "absolute"

        // Calculate distance from bottom left corner
        const dx = x - 0 // bottomLeftX is 0
        const dy = rows - 1 - y // bottomLeftY is rows - 1
        const distance = dx + dy + Math.random() * 0.5

        tile.style.cssText = `
          width: ${tileSize}px;
          height: ${tileSize}px;
          left: ${x * tileSize}px;
          top: ${y * tileSize}px;
          overflow: hidden;
          opacity: 0;
          transition: opacity 0.2s ease-out;
          transition-delay: ${(distance * duration) / (rows + cols) + 500}ms;
        `

        // Create SVG wrapper positioned to show correct tile portion
        const svgWrapper = document.createElement("div")
        svgWrapper.style.cssText = `
          position: absolute;
          left: ${-x * tileSize}px;
          top: ${-y * tileSize}px;
          width: 1920px;
          height: 1080px;
        `

        // Render the inline SVG into the wrapper
        render(() => <InlinePatternSVG />, svgWrapper)

        tile.appendChild(svgWrapper)
        container.appendChild(tile)
        tiles.push({ element: tile, x, y, distance })
      }
    }

    tiles.sort((a, b) => (a.distance || 0) - (b.distance || 0))

    // Trigger reflow to ensure transitions work
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    container.offsetHeight

    tiles.forEach((tile) => {
      tile.element.style.opacity = "1"
    })

    onCleanup(() => {
      container.remove()
    })
  })

  return (
    <div
      class="relative h-screen w-screen"
      style={{
        "--pattern-background": "21 24 30",
        "--pattern-fill": "30 33 41"
      }}
    >
      <div ref={containerRef} class="absolute inset-0" />
      <div class="relative">{props.children}</div>
    </div>
  )
}

export default PatternBackground
