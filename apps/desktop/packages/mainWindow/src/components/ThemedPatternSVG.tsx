import { onMount } from "solid-js"
import patternSvgRaw from "/assets/images/gdlauncher_pattern.svg?raw"

const ThemedPatternSVG = () => {
  let containerRef: HTMLDivElement | undefined

  onMount(() => {
    try {
      // Import SVG at build time (works in both dev and production)
      let svgText = patternSvgRaw

      // Replace fill attributes with CSS classes
      svgText = svgText.replace(
        /fill="rgb\(var\(--pattern-background\)\)"/g,
        'class="pattern-bg"'
      )
      svgText = svgText.replace(
        /fill="rgb\(var\(--pattern-fill\)\)"/g,
        'class="pattern-fill"'
      )

      // Remove fixed width/height attributes and make SVG responsive
      svgText = svgText.replace(/width="[^"]*"/, "")
      svgText = svgText.replace(/height="[^"]*"/, "")
      svgText = svgText.replace(
        /<svg([^>]*)>/,
        '<svg$1 style="width: 100%; height: 100%;" preserveAspectRatio="xMidYMid slice">'
      )

      // Parse and sanitize SVG content
      const parser = new DOMParser()
      const doc = parser.parseFromString(svgText, "image/svg+xml")
      const svgElement = doc.querySelector("svg")

      if (svgElement && containerRef) {
        containerRef.appendChild(svgElement)
      }
    } catch (error) {
      console.error("Failed to load pattern SVG:", error)
    }
  })

  return (
    <>
      <style>{`
        .pattern-bg {
          fill: rgb(var(--darkSlate-900));
        }
        .pattern-fill {
          fill: rgb(var(--darkSlate-800));
        }
      `}</style>
      <div ref={containerRef} class="h-full w-full" />
    </>
  )
}

export default ThemedPatternSVG
