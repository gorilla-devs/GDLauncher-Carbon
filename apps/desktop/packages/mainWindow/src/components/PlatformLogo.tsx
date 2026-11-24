import { Show, createUniqueId } from "solid-js"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"

export type Platform = "curseforge" | "modrinth"

interface Props {
  platform: Platform
  class?: string
  alt?: string
  /** The CurseForge mark's colour class. By default white on a dark
   *  palette and the text colour on a light one (themes/controls.css); pass
   *  `text-white` where it sits on a fixed dark backing. */
  color?: string
}

/**
 * The CurseForge or Modrinth logo (`platform-logo`, with
 * `platform-logo.curseforge` or `platform-logo.modrinth`).
 *
 * The CurseForge mark is drawn inline in `currentColor`: white on a dark
 * palette, as the old white picture was, and the palette's text colour on a
 * light one, where the white one vanished. A theme recolours it with
 * `color`. The Modrinth logo keeps its own green.
 */
export const PlatformLogo = (props: Props) => {
  const clip = createUniqueId()
  return (
    <Show
      when={props.platform === "curseforge"}
      fallback={
        <img
          data-gdl="platform-logo platform-logo.modrinth"
          src={ModrinthLogo}
          class={props.class}
          alt={props.alt}
        />
      }
    >
      <svg
        data-gdl="platform-logo platform-logo.curseforge"
        class={`${props.color ?? "gdl-white-text"} ${props.class ?? ""}`}
        width="114"
        height="73"
        viewBox="0 0 114 73"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        role="img"
        aria-label={props.alt ?? "CurseForge"}
      >
        <g clip-path={`url(#${clip})`}>
          <path
            d="M86.9804 22.9535C86.9804 22.9535 110.238 19.2696 113.911 8.52538H78.2827V0H0L9.64353 11.2401V22.7565C9.64353 22.7565 33.9764 21.4856 43.3888 28.655C56.2728 40.6503 28.898 56.8651 28.898 56.8651L24.2038 72.4678C31.5443 65.4472 45.5345 56.365 71.1848 56.8028C61.4236 59.9015 51.6085 64.7416 43.9674 72.4678H95.8198L90.937 56.8651C90.937 56.8651 53.3556 34.6059 86.9804 22.9549V22.9535Z"
            fill="currentColor"
          />
        </g>
        <defs>
          <clipPath id={clip}>
            <rect width="113.911" height="72.4678" fill="white" />
          </clipPath>
        </defs>
      </svg>
    </Show>
  )
}
