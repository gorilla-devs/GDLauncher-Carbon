import { splitProps } from "solid-js"

type ColorVariant =
  | "primary"
  | "emerald"
  | "blue"
  | "purple"
  | "cyan"
  | "pink"
  | "yellow"
  | "red"

interface Props {
  /** Icon class name (e.g., "i-hugeicons:rocket-01") */
  icon: string
  /** Color variant for the icon and container */
  color?: ColorVariant
  /** Background style: "colored" (semi-transparent color) or "subtle" (dark slate) */
  variant?: "colored" | "subtle"
  /** Whether to show the glow/shadow effect (only applies to "colored" variant) */
  glow?: boolean
  /** Size variant */
  size?: "sm" | "md" | "lg"
  /** Additional class names */
  class?: string
}

const colorStyles: Record<
  ColorVariant,
  { bg: string; border: string; text: string; shadow: string }
> = {
  primary: {
    bg: "bg-primary-500/10",
    border: "border-primary-500/30",
    text: "text-primary-500",
    shadow: "shadow-[0_0_30px_rgba(139,92,246,0.15)]"
  },
  emerald: {
    bg: "bg-emerald-500/10",
    border: "border-emerald-500/30",
    text: "text-emerald-500",
    shadow: "shadow-[0_0_30px_rgba(16,185,129,0.15)]"
  },
  blue: {
    bg: "bg-blue-500/10",
    border: "border-blue-500/30",
    text: "text-blue-500",
    shadow: "shadow-[0_0_30px_rgba(59,130,246,0.15)]"
  },
  purple: {
    bg: "bg-purple-500/10",
    border: "border-purple-500/30",
    text: "text-purple-500",
    shadow: "shadow-[0_0_30px_rgba(168,85,247,0.15)]"
  },
  cyan: {
    bg: "bg-cyan-500/10",
    border: "border-cyan-500/30",
    text: "text-cyan-500",
    shadow: "shadow-[0_0_30px_rgba(6,182,212,0.15)]"
  },
  pink: {
    bg: "bg-pink-500/10",
    border: "border-pink-500/30",
    text: "text-pink-500",
    shadow: "shadow-[0_0_30px_rgba(236,72,153,0.15)]"
  },
  yellow: {
    bg: "bg-yellow-500/10",
    border: "border-yellow-500/30",
    text: "text-yellow-500",
    shadow: "shadow-[0_0_30px_rgba(234,179,8,0.15)]"
  },
  red: {
    bg: "bg-red-500/10",
    border: "border-red-500/30",
    text: "text-red-500",
    shadow: "shadow-[0_0_30px_rgba(239,68,68,0.15)]"
  }
}

const sizeStyles: Record<
  "sm" | "md" | "lg",
  { container: string; icon: string }
> = {
  sm: { container: "h-14 w-14 rounded-xl", icon: "text-[1.75rem]" },
  md: { container: "h-20 w-20 rounded-2xl", icon: "text-[2.5rem]" },
  lg: { container: "h-24 w-24 rounded-2xl", icon: "text-[3rem]" }
}

/**
 * A styled icon container for hero/featured sections in modals and pages.
 *
 * @example
 * // Colored background with glow (default)
 * <HeroIcon icon="i-hugeicons:rocket-01" color="primary" glow />
 *
 * // Subtle dark background (for cards)
 * <HeroIcon icon="i-hugeicons:sparkles" color="emerald" variant="subtle" />
 */
export function HeroIcon(props: Props) {
  const [local, others] = splitProps(props, [
    "icon",
    "color",
    "variant",
    "glow",
    "size",
    "class"
  ])

  const color = () => local.color ?? "primary"
  const size = () => local.size ?? "md"
  const variant = () => local.variant ?? "colored"
  const showGlow = () => local.glow ?? variant() === "colored"

  const styles = () => colorStyles[color()]
  const sizeStyle = () => sizeStyles[size()]

  const bgClass = () =>
    variant() === "colored" ? styles().bg : "bg-darkSlate-700"

  const borderClass = () =>
    variant() === "colored"
      ? `border-2 ${styles().border}`
      : `border ${styles().border}`

  return (
    <div
      class={`flex items-center justify-center ${bgClass()} ${borderClass()} ${sizeStyle().container} ${showGlow() ? styles().shadow : ""} ${local.class ?? ""}`}
      {...others}
    >
      <i class={`${local.icon} ${styles().text} ${sizeStyle().icon}`} />
    </div>
  )
}
