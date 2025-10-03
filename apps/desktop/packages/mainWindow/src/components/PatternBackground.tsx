import { createEffect, onCleanup } from "solid-js"

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
        tile.className = "absolute bg-pattern opacity-0"

        // Calculate distance from bottom left corner
        const dx = x - 0 // bottomLeftX is 0
        const dy = rows - 1 - y // bottomLeftY is rows - 1
        const distance = dx + dy + Math.random() * 0.5

        tile.style.cssText = `
          width: ${tileSize}px;
          height: ${tileSize}px;
          left: ${x * tileSize}px;
          top: ${y * tileSize}px;
          background-image: url(./assets/images/gdlauncher_pattern.svg);
          background-position: ${-x * tileSize}px ${-y * tileSize}px;
          transition: opacity 0.2s ease-out;
          transition-delay: ${(distance * duration) / (rows + cols) + 500}ms;
        `
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
    <div class="relative h-screen w-screen">
      <div ref={containerRef} class="absolute inset-0" />
      <div class="relative">{props.children}</div>
    </div>
  )
}

export default PatternBackground
