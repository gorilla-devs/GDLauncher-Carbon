import { createSignal, onMount, For, Show } from "solid-js"
import { detectOS } from "../../utils/detectOS"
import type { OS } from "../../utils/detectOS"

const OS_CONFIG = {
  Windows: {
    label: "Windows",
    requirement: "Windows 10+",
    icon: "i-simple-icons:windows11",
    url: "/download/windows"
  },
  MacOS: {
    label: "macOS",
    requirement: "macOS 10.15+",
    icon: "i-simple-icons:apple",
    url: "/download/mac"
  },
  Linux: {
    label: "Linux",
    requirement: "Linux (glibc 2.31+)",
    icon: "i-simple-icons:linux",
    url: "/download/linux"
  }
} as const

export default function HeroDownload() {
  const [currentOS, setCurrentOS] = createSignal<OS>("Windows")
  const [version, setVersion] = createSignal<string | null>(null)

  onMount(async () => {
    setCurrentOS(detectOS())

    // Fetch version from API
    try {
      const response = await fetch("/api/version")
      const data = await response.json()
      setVersion(data.version)
    } catch {
      setVersion("latest")
    }
  })

  const config = () => OS_CONFIG[currentOS()]

  return (
    <div class="flex flex-col items-start gap-4">
      {/* Main Download Button */}
      <a
        href={config().url}
        data-astro-prefetch="false"
        class="group relative inline-flex items-center gap-3 px-6 py-3.5 rounded-lg bg-primary-500 hover:bg-primary-600 text-lightSlate-50 font-semibold transition-all duration-200 shadow-lg shadow-primary-500/25 hover:shadow-primary-500/40 active:scale-95 ease-spring"
      >
        <div class={`${config().icon} w-5 h-5`}></div>
        <span>Download for {config().label}</span>
      </a>

      {/* Version and Requirements */}
      <p class="text-sm text-darkSlate-100 flex items-center gap-2"><Show when={version()} fallback={<span class="text-lightSlate-200">Loading...</span>}><span class="text-lightSlate-200">v{version()}</span></Show><span class="text-darkSlate-300">|</span><span>{config().requirement}</span></p>

      {/* All OS Icons */}
      <div class="flex items-center gap-4">
        <span class="text-xs text-darkSlate-200 uppercase tracking-wider">Also available for</span>
        <div class="flex items-center gap-2">
          <For each={Object.entries(OS_CONFIG)}>
            {([_os, cfg]) => (
              <a
                href={cfg.url}
                title={`Download for ${cfg.label}`}
                aria-label={`Download for ${cfg.label}`}
                data-astro-prefetch="false"
                class="p-2 rounded-lg transition-all duration-200 text-darkSlate-200 hover:text-lightSlate-100 hover:bg-darkSlate-700/50 active:scale-90 ease-spring"
              >
                <div class={`${cfg.icon} w-5 h-5`} aria-hidden="true"></div>
              </a>
            )}
          </For>
        </div>
      </div>
    </div>
  )
}
