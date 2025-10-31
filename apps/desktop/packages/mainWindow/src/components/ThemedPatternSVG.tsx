import { createSignal, onMount, Show } from "solid-js"
import patternSvgRaw from "/assets/images/gdlauncher_pattern.svg?raw"

const ThemedPatternSVG = () => {
  const [svgContent, setSvgContent] = createSignal<string>("")

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

      setSvgContent(svgText)
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
      <Show when={svgContent()}>
        <div class="h-full w-full" innerHTML={svgContent()} />
      </Show>
    </>
  )
}

export default ThemedPatternSVG
