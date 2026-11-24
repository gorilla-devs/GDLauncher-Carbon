import { JSX } from "solid-js"

export type TextLinkTone = "bright" | "muted"

const base =
  "rounded text-sm underline underline-offset-2 outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500 disabled:opacity-50 disabled:cursor-not-allowed"

const tones: Record<TextLinkTone, string> = {
  bright: "text-lightSlate-100 hover:text-lightSlate-50",
  muted: "text-lightSlate-700 hover:text-lightSlate-50"
}

/** The one recipe for a quiet text link, for elements that can't be a plain
 *  `TextLink` (a Kobalte trigger, an anchor). */
export const textLinkClass = (tone: TextLinkTone): string =>
  `${base} ${tones[tone]}`

/** A quiet underlined link that does something, such as a dialog's "Cancel"
 *  beside its one primary button. */
export const TextLink = (props: {
  tone?: TextLinkTone
  disabled?: boolean
  onClick?: () => void
  ref?: (el: HTMLButtonElement) => void
  class?: string
  "data-testid"?: string
  children: JSX.Element
}) => (
  <button
    ref={props.ref}
    type="button"
    data-testid={props["data-testid"]}
    disabled={props.disabled}
    class={`${textLinkClass(props.tone ?? "bright")}${
      props.class ? ` ${props.class}` : ""
    }`}
    onClick={() => props.onClick?.()}
  >
    {props.children}
  </button>
)
