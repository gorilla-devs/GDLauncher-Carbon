import { mergeProps } from "solid-js"

interface Props {
  class?: string
  value?: number
  max?: number
  indeterminate?: boolean
  color?: string
  size?: "small" | "medium" | "large"
  variant?: "rounded" | "square"
}

export const Progress = (props: Props) => {
  const mergedProps = mergeProps(
    {
      max: 100,
      size: "medium",
      variant: "rounded",
      color: "bg-primary-500"
    },
    props
  )

  const isIndeterminate = () => props.indeterminate || props.value === undefined
  const percentage = () => {
    if (isIndeterminate()) return 0
    const max = mergedProps.max
    const value = Math.min(Math.max(props.value || 0, 0), max)
    return (value / max) * 100
  }

  const sizeClasses = () => {
    switch (mergedProps.size) {
      case "small":
        return "h-1"
      case "large":
        return "h-4"
      default:
        return "h-2"
    }
  }

  const containerClasses = () => {
    const baseClasses = `bg-darkSlate-500 w-full overflow-hidden ${sizeClasses()}`
    const roundingClasses =
      mergedProps.variant === "rounded" ? "rounded-full" : "rounded-sm"
    return `${baseClasses} ${roundingClasses} ${props.class || ""}`
  }

  const barClasses = () => {
    const baseClasses = `h-full transition-all duration-400 ease-spring ${mergedProps.color}`
    const animationClasses = isIndeterminate()
      ? "w-full origin-[0%_50%] animate-loadingbar"
      : ""
    return `${baseClasses} ${animationClasses}`
  }

  return (
    <div class={containerClasses()}>
      <div
        class={barClasses()}
        style={isIndeterminate() ? {} : { width: `${percentage()}%` }}
      />
    </div>
  )
}
