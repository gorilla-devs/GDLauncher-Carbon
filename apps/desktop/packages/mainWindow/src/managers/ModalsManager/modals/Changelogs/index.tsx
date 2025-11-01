import { ModalProps } from "../../"
import ModalLayout from "../../ModalLayout"

import { Trans } from "@gd/i18n"
import { For, Show, createSignal, onMount, createMemo } from "solid-js"
import changelogs, { Changelog, ChangelogEntry } from "./changelogs"
import { Button, Badge } from "@gd/ui"
import { rspc } from "@/utils/rspcClient"

type CategoryType = keyof Changelog

interface FeatureCardProps {
  entry: ChangelogEntry
  type: CategoryType
  index: number
}

const FeatureCard = (props: FeatureCardProps) => {
  const getColor = () => {
    switch (props.type) {
      case "new":
        return {
          text: "text-green-400",
          bg: "bg-green-500/10",
          border: "border-green-500/20",
          icon: "i-hugeicons:sparkles"
        }
      case "improved":
        return {
          text: "text-yellow-400",
          bg: "bg-yellow-500/10",
          border: "border-yellow-500/20",
          icon: "i-hugeicons:magic-wand-01"
        }
      case "fixed":
        return {
          text: "text-red-400",
          bg: "bg-red-500/10",
          border: "border-red-500/20",
          icon: "i-hugeicons:bug-01"
        }
    }
  }

  return (
    <div
      class={`${getColor().bg} ${getColor().border} group relative overflow-hidden rounded-xl border p-4 transition-all duration-300 hover:scale-[1.01] hover:shadow-lg`}
      style={{
        animation: `fadeInUp 0.4s ease-out ${props.index * 0.05}s both`
      }}
    >
      <div class="flex items-start gap-3">
        <div
          class={`${getColor().icon} ${getColor().text} mt-1 shrink-0 h-5 w-5`}
        />
        <div class="flex-1">
          <h3 class="text-lightSlate-50 m-0 mb-2 text-base font-semibold">
            {props.entry.title}
          </h3>
          <Show when={props.entry.description}>
            <p class="text-lightSlate-500 m-0 text-sm leading-6">
              {props.entry.description}
            </p>
          </Show>
        </div>
      </div>
    </div>
  )
}

interface HeroFeatureCardProps {
  entry: ChangelogEntry
  type: CategoryType
}

const HeroFeatureCard = (props: HeroFeatureCardProps) => {
  const [mediaLoaded, setMediaLoaded] = createSignal(false)

  const getGradient = () => {
    switch (props.type) {
      case "new":
        return "from-primary-500/20 to-primary-600/5"
      case "improved":
        return "from-yellow-500/20 to-yellow-600/5"
      case "fixed":
        return "from-red-500/20 to-red-600/5"
    }
  }

  const getIconColor = () => {
    switch (props.type) {
      case "new":
        return "text-primary-400"
      case "improved":
        return "text-yellow-400"
      case "fixed":
        return "text-red-400"
    }
  }

  const getIcon = () => {
    switch (props.type) {
      case "new":
        return "i-hugeicons:sparkles"
      case "improved":
        return "i-hugeicons:rocket-02"
      case "fixed":
        return "i-hugeicons:checkmark-badge-01"
    }
  }

  const isVideo = () => {
    const media = props.entry.media
    return (
      media &&
      (media.endsWith(".mp4") ||
        media.endsWith(".webm") ||
        media.endsWith(".mov"))
    )
  }

  return (
    <div
      class={`bg-gradient-to-br ${getGradient()} relative overflow-hidden rounded-2xl border border-primary-500/20`}
      style={{
        animation: "fadeInScale 0.5s ease-out both"
      }}
    >
      <div
        class="relative z-10"
        classList={{
          "p-6": !props.entry.media,
          "grid grid-cols-1 md:grid-cols-2 gap-6": !!props.entry.media
        }}
      >
        {/* Content Section */}
        <div
          classList={{
            "p-6": !!props.entry.media
          }}
        >
          <div class="mb-4 flex items-center gap-3">
            <div class={`${getIcon()} ${getIconColor()} h-8 w-8`} />
            <Badge variant="secondary" class="text-xs font-semibold">
              Major Feature
            </Badge>
          </div>
          <h2 class="text-lightSlate-50 mb-3 text-2xl font-bold">
            {props.entry.title}
          </h2>
          <Show when={props.entry.description}>
            <p class="text-lightSlate-300 text-base leading-7">
              {props.entry.description}
            </p>
          </Show>
        </div>

        {/* Media Section */}
        <Show when={props.entry.media}>
          <div class="relative overflow-hidden rounded-xl p-6">
            <div class="relative aspect-[4/3] w-full">
              {/* Loading skeleton - absolute positioned as background */}
              <Show when={!mediaLoaded()}>
                <div class="bg-darkSlate-700 absolute inset-0 animate-pulse rounded-lg" />
              </Show>

              {/* Media - absolute positioned on top */}
              <Show
                when={isVideo()}
                fallback={
                  <img
                    src={props.entry.media}
                    alt={props.entry.title}
                    class="absolute inset-0 h-full w-full rounded-lg object-cover shadow-lg transition-opacity duration-500"
                    classList={{
                      "opacity-0": !mediaLoaded(),
                      "opacity-100": mediaLoaded()
                    }}
                    onLoad={() => setMediaLoaded(true)}
                  />
                }
              >
                <video
                  src={props.entry.media}
                  autoplay
                  loop
                  muted
                  playsinline
                  class="absolute inset-0 h-full w-full rounded-lg object-cover shadow-lg transition-opacity duration-500"
                  classList={{
                    "opacity-0": !mediaLoaded(),
                    "opacity-100": mediaLoaded()
                  }}
                  onLoadedData={() => setMediaLoaded(true)}
                />
              </Show>
            </div>
          </div>
        </Show>
      </div>

      {/* Decorative background element - only show when no media */}
      <Show when={!props.entry.media}>
        <div
          class={`${getIcon()} ${getIconColor()} absolute -right-8 -top-8 opacity-10 h-32 w-32`}
        />
      </Show>
    </div>
  )
}

interface CategoryFilterProps {
  type: CategoryType
  active: boolean
  count: number
  onToggle: () => void
}

const CategoryFilter = (props: CategoryFilterProps) => {
  const getConfig = () => {
    switch (props.type) {
      case "new":
        return {
          label: "New Features",
          icon: "i-hugeicons:sparkles",
          activeColor: "bg-green-500 text-white",
          inactiveColor:
            "bg-darkSlate-700 text-lightSlate-500 hover:bg-darkSlate-600"
        }
      case "improved":
        return {
          label: "Improvements",
          icon: "i-hugeicons:magic-wand-01",
          activeColor: "bg-yellow-500 text-darkSlate-900",
          inactiveColor:
            "bg-darkSlate-700 text-lightSlate-500 hover:bg-darkSlate-600"
        }
      case "fixed":
        return {
          label: "Bug Fixes",
          icon: "i-hugeicons:bug-01",
          activeColor: "bg-red-500 text-white",
          inactiveColor:
            "bg-darkSlate-700 text-lightSlate-500 hover:bg-darkSlate-600"
        }
    }
  }

  const config = getConfig()

  return (
    <button
      class={`flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-all duration-200 ${
        props.active ? config.activeColor : config.inactiveColor
      }`}
      onClick={props.onToggle}
    >
      <div class={`${config.icon} h-4 w-4`} />
      <span>{config.label}</span>
      <Badge
        variant={props.active ? "default" : "secondary"}
        class="ml-1 min-w-[1.5rem] text-xs"
      >
        {props.count}
      </Badge>
    </button>
  )
}

const Changelogs = (props: ModalProps) => {
  const sendEvent = rspc.createMutation(() => ({
    mutationKey: ["metrics.sendEvent"]
  }))

  // State for filters
  const [activeFilters, setActiveFilters] = createSignal<Set<CategoryType>>(
    new Set(["new", "improved", "fixed"])
  )

  onMount(() => {
    sendEvent.mutate({
      event_name: "changelog_viewed"
    })
  })

  const toggleFilter = (type: CategoryType) => {
    setActiveFilters((prev) => {
      const newSet = new Set(prev)
      if (newSet.has(type)) {
        newSet.delete(type)
      } else {
        newSet.add(type)
      }
      return newSet
    })
  }

  // Get hero feature (first new feature)
  const heroFeature = createMemo(() => changelogs.new[0])

  // Get remaining features based on active filters
  const getFilteredFeatures = () => {
    const features: { entry: ChangelogEntry; type: CategoryType }[] = []

    // Skip first new feature (it's the hero)
    if (activeFilters().has("new")) {
      changelogs.new.slice(1).forEach((entry) => {
        features.push({ entry, type: "new" })
      })
    }

    if (activeFilters().has("improved")) {
      changelogs.improved.forEach((entry) => {
        features.push({ entry, type: "improved" })
      })
    }

    if (activeFilters().has("fixed")) {
      changelogs.fixed.forEach((entry) => {
        features.push({ entry, type: "fixed" })
      })
    }

    return features
  }

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      noPadding
      height="h-[700px] max-h-[90vh]"
      width="w-[80vw] max-w-[900px]"
    >
      <style>
        {`
          @keyframes fadeInUp {
            from {
              opacity: 0;
              transform: translateY(20px);
            }
            to {
              opacity: 1;
              transform: translateY(0);
            }
          }

          @keyframes fadeInScale {
            from {
              opacity: 0;
              transform: scale(0.95);
            }
            to {
              opacity: 1;
              transform: scale(1);
            }
          }

          .line-clamp-2 {
            display: -webkit-box;
            -webkit-line-clamp: 2;
            -webkit-box-orient: vertical;
            overflow: hidden;
          }
        `}
      </style>

      <div class="box-border flex h-full w-full flex-col overflow-hidden">
        {/* Header */}
        <div class="border-darkSlate-600 border-b px-8 py-6">
          <h2 class="text-lightSlate-50 m-0 text-2xl font-bold">
            <Trans
              key="changelogs.whats_new_in"
              options={{
                version: __APP_VERSION__
              }}
            />
          </h2>
          <p class="text-lightSlate-500 mt-2 text-sm">
            Discover the latest features, improvements, and fixes
          </p>
        </div>

        {/* Scrollable Content */}
        <div class="flex-1 overflow-y-auto overflow-x-hidden px-8 py-6">
          {/* Hero Feature */}
          <Show when={heroFeature()}>
            {(hero) => (
              <div class="mb-8">
                <HeroFeatureCard entry={hero()} type="new" />
              </div>
            )}
          </Show>

          {/* Category Filters */}
          <div class="mb-6 flex flex-wrap gap-3">
            <CategoryFilter
              type="new"
              active={activeFilters().has("new")}
              count={changelogs.new.length}
              onToggle={() => toggleFilter("new")}
            />
            <CategoryFilter
              type="improved"
              active={activeFilters().has("improved")}
              count={changelogs.improved.length}
              onToggle={() => toggleFilter("improved")}
            />
            <CategoryFilter
              type="fixed"
              active={activeFilters().has("fixed")}
              count={changelogs.fixed.length}
              onToggle={() => toggleFilter("fixed")}
            />
          </div>

          {/* Feature Cards */}
          <div class="mb-6 flex flex-col gap-3">
            <For each={getFilteredFeatures()}>
              {(feature, index) => (
                <FeatureCard
                  entry={feature.entry}
                  type={feature.type}
                  index={index()}
                />
              )}
            </For>
          </div>

          {/* Community & Support CTAs */}
          <div class="mt-8 grid grid-cols-1 gap-4 md:grid-cols-2">
            {/* Discord CTA */}
            <div class="border-darkSlate-600 flex flex-col items-center gap-4 rounded-xl border p-6">
              <div class="text-center">
                <h3 class="text-lightSlate-50 mb-2 text-lg font-semibold">
                  <Trans key="changelogs.cta_discord_title" />
                </h3>
                <p class="text-lightSlate-500 text-sm">
                  <Trans key="changelogs.cta_discord_description" />
                </p>
              </div>
              <Button
                backgroundColor="bg-brands-discord"
                onClick={() => {
                  window.open("https://discord.gdlauncher.com", "_blank")
                }}
              >
                <div class="flex items-center justify-center gap-2">
                  <div class="i-hugeicons:discord inline-block h-5 w-5" />
                  <Trans key="changelogs.cta_discord_button" />
                </div>
              </Button>
            </div>

            {/* GitHub CTA */}
            <div class="border-darkSlate-600 flex flex-col items-center gap-4 rounded-xl border p-6">
              <div class="text-center">
                <h3 class="text-lightSlate-50 mb-2 text-lg font-semibold">
                  <Trans key="changelogs.cta_github_title" />
                </h3>
                <p class="text-lightSlate-500 text-sm">
                  <Trans key="changelogs.cta_github_description" />
                </p>
              </div>
              <Button
                type="secondary"
                onClick={() => {
                  window.open(
                    "https://github.com/gorilla-devs/GDLauncher-Carbon",
                    "_blank"
                  )
                }}
              >
                <div class="flex items-center justify-center gap-2">
                  <div class="i-hugeicons:github inline-block h-5 w-5" />
                  <Trans key="changelogs.cta_github_button" />
                </div>
              </Button>
            </div>
          </div>

          {/* Report Issue CTA */}
          <div class="border-darkSlate-600 mt-4 flex items-center justify-between gap-4 rounded-xl border p-6">
            <div>
              <h3 class="text-lightSlate-50 mb-1 text-base font-semibold">
                <Trans key="changelogs.cta_report_title" />
              </h3>
              <p class="text-lightSlate-500 text-sm">
                <Trans key="changelogs.cta_report_description" />
              </p>
            </div>
            <Button
              type="outline"
              onClick={() => {
                window.open(
                  "https://github.com/gorilla-devs/GDLauncher-Carbon/issues/new",
                  "_blank"
                )
              }}
            >
              <div class="flex items-center justify-center gap-2">
                <div class="i-hugeicons:alert-02 inline-block h-5 w-5" />
                <Trans key="changelogs.cta_report_button" />
              </div>
            </Button>
          </div>
        </div>
      </div>
    </ModalLayout>
  )
}

export default Changelogs
