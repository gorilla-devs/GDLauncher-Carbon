import { For, Show, createMemo } from "solid-js"
import { Trans } from "@gd/i18n"
import { Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import {
  getAuthorColor,
  getInitials,
  getContrastTextColor
} from "@/utils/avatarHelpers"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"

export interface Author {
  name: string
  avatarUrl?: string | null
  url?: string | null
  role?: string | null
  id: string | number
  platform: "curseforge" | "modrinth"
}

interface AuthorAvatarsProps {
  authors: Author[]
  maxDisplay?: number
  size?: "sm" | "md" | "lg"
}

const AuthorAvatars = (props: AuthorAvatarsProps) => {
  const maxDisplay = () => props.maxDisplay ?? 4
  const size = () => props.size ?? "md"

  const sizeClasses = createMemo(() => {
    switch (size()) {
      case "sm":
        return {
          avatar: "w-4 h-4",
          text: "text-xs",
          overlap: "-ml-0.5"
        }
      case "lg":
        return {
          avatar: "w-6 h-6",
          text: "text-xs",
          overlap: "-ml-1"
        }
      default: // md
        return {
          avatar: "w-5 h-5",
          text: "text-xs",
          overlap: "-ml-1"
        }
    }
  })

  const displayedAuthors = createMemo(() =>
    props.authors.slice(0, maxDisplay())
  )

  const overflowCount = createMemo(() =>
    Math.max(0, props.authors.length - maxDisplay())
  )

  const handleAuthorClick = (author: Author) => {
    // Only allow clicking for Modrinth authors for now
    if (author.platform === "modrinth" && author.url) {
      window.openExternalLink(author.url)
    }
  }

  const renderAuthorAvatar = (author: Author, index: number) => {
    const backgroundColor = getAuthorColor(author.id.toString())
    const textColor = getContrastTextColor(backgroundColor)
    const initials = getInitials(author.name)

    return (
      <Tooltip>
        <TooltipTrigger>
          <div
            class={`${sizeClasses().avatar} ${index > 0 ? sizeClasses().overlap : ""} relative z-10 transition-all duration-200 hover:z-20 hover:scale-110`}
            style={{ "z-index": `${10 + index}` }}
            onClick={() => handleAuthorClick(author)}
          >
            {/* Transparent cutout ring */}
            <div
              class="border-darkSlate-800 absolute rounded-full border-4"
              style={{
                width: `calc(${sizeClasses().avatar.split(" ")[0].replace("w-", "")} * 0.25rem + 8px)`,
                height: `calc(${sizeClasses().avatar.split(" ")[1].replace("h-", "")} * 0.25rem + 8px)`,
                left: "-4px",
                top: "-4px",
                "z-index": "-1"
              }}
            />
            <Show
              when={author.avatarUrl}
              fallback={
                <div
                  class={`${sizeClasses().avatar} flex items-center justify-center rounded-full font-bold ${sizeClasses().text}`}
                  style={{
                    "background-color": backgroundColor,
                    color: textColor
                  }}
                >
                  {initials}
                </div>
              }
            >
              <img
                src={author.avatarUrl!}
                alt={`${author.name} avatar`}
                class={`${sizeClasses().avatar} rounded-full object-cover`}
                loading="lazy"
              />
            </Show>
          </div>
        </TooltipTrigger>
        <TooltipContent>
          <div class="flex min-w-0 flex-col gap-2">
            <div class="flex items-center gap-2">
              <img
                src={
                  author.platform === "curseforge"
                    ? CurseforgeLogo
                    : ModrinthLogo
                }
                class="h-4 w-4"
                alt={author.platform}
              />
              <span class="text-lightSlate-50 font-semibold">
                {author.name}
              </span>
            </div>
            <Show when={author.role}>
              <div class="text-lightSlate-400 text-xs">
                <Trans
                  key="content:_trn_author_role"
                  options={{ role: author.role }}
                />
              </div>
            </Show>
            <Show when={author.platform === "modrinth" && author.url}>
              <div class="text-xs text-blue-400">
                <Trans key="content:_trn_author_click_to_view" />
              </div>
            </Show>
          </div>
        </TooltipContent>
      </Tooltip>
    )
  }

  const renderOverflowIndicator = () => {
    const backgroundColor = getAuthorColor("overflow")
    const textColor = getContrastTextColor(backgroundColor)

    return (
      <div
        class={`${sizeClasses().avatar} ${sizeClasses().overlap} border-darkSlate-800 relative z-10 flex items-center justify-center rounded-full border-2 font-bold ${sizeClasses().text}`}
        style={{
          "background-color": backgroundColor,
          color: textColor,
          "z-index": `${10 + maxDisplay()}`
        }}
      >
        +{overflowCount()}
      </div>
    )
  }

  return (
    <Show when={props.authors.length > 0}>
      <div class="flex items-center">
        <div class="i-hugeicons:user text-lightSlate-600 mr-2 text-lg" />
        <div class="flex items-center">
          <For each={displayedAuthors()}>
            {(author, index) => renderAuthorAvatar(author, index())}
          </For>
          <Show when={overflowCount() > 0}>{renderOverflowIndicator()}</Show>
        </div>
      </div>
    </Show>
  )
}

export default AuthorAvatars
