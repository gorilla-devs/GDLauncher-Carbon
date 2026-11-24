import { onMount } from "solid-js"
import patternSvgRaw from "/assets/images/gdlauncher_pattern.svg?raw"
import { themedPatternSvg } from "@/themes/patternSvg"

const ThemedPatternSVG = () => {
  let containerRef: HTMLDivElement | undefined

  onMount(() => {
    try {
      // The SVG is imported at build time (works in dev and production).
      const svgElement = themedPatternSvg(patternSvgRaw)
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
        /* The logos take the colour of their hooked layer
           (ad-column-background.pattern); the marks cut into them keep the
           backdrop's default colour. */
        .pattern-bg {
          fill: rgb(var(--darkSlate-900));
        }
        .pattern-fill {
          fill: currentColor;
        }
      `}</style>
      <div ref={containerRef} class="h-full w-full" />
    </>
  )
}

export default ThemedPatternSVG
