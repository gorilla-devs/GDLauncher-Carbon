interface Props {
  class?: string
  value?: number
  max?: number
  indeterminate?: boolean
}

export const LoadingBar = (props: Props) => {
  const isIndeterminate = () => props.indeterminate || props.value === undefined
  const percentage = () => {
    if (isIndeterminate()) return 0
    const max = props.max || 100
    const value = Math.min(Math.max(props.value || 0, 0), max)
    return (value / max) * 100
  }

  return (
    <div
      class={`h-2 bg-darkSlate-500 w-full overflow-hidden rounded-full ${
        props.class || ""
      }`}
    >
      <div 
        class={`h-full bg-primary-500 transition-all duration-300 ease-out ${
          isIndeterminate() 
            ? "w-full origin-[0%_50%] animate-loadingbar" 
            : ""
        }`}
        style={
          isIndeterminate() 
            ? {} 
            : { width: `${percentage()}%` }
        }
      />
    </div>
  )
}
