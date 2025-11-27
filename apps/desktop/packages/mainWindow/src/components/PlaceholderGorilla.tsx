import { Trans } from "@gd/i18n"

interface PlaceholderGorillaProps {
  size?: number // Size in rem (default: 10rem = 160px)
  variant: string // Label describing the illustration type
}

/**
 * Temporary placeholder component for gorilla mascot illustrations
 * Shows a simple gorilla silhouette with a label indicating the intended expression/pose
 *
 * This will be replaced with actual mascot illustrations from designers
 *
 * NOTE: Only visible in development mode - hidden in production builds
 */
export function PlaceholderGorilla(props: PlaceholderGorillaProps) {
  // Hide in production - only show placeholders in development
  if (import.meta.env.PROD) {
    return null
  }

  const size = props.size || 10

  return (
    <div class="flex flex-col items-center gap-4">
      {/* SVG Gorilla Silhouette */}
      <svg
        width={`${size}rem`}
        height={`${size}rem`}
        viewBox="0 0 200 200"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        class="opacity-30"
      >
        {/* Gorilla Head Shape */}
        <ellipse cx="100" cy="100" rx="70" ry="75" fill="currentColor" />

        {/* Ears */}
        <circle cx="50" cy="70" r="25" fill="currentColor" />
        <circle cx="150" cy="70" r="25" fill="currentColor" />

        {/* Face Area (lighter) */}
        <ellipse
          cx="100"
          cy="120"
          rx="50"
          ry="40"
          fill="currentColor"
          opacity="0.6"
        />

        {/* Eyes */}
        <circle cx="80" cy="100" r="8" fill="currentColor" opacity="0.8" />
        <circle cx="120" cy="100" r="8" fill="currentColor" opacity="0.8" />

        {/* Nose/Snout */}
        <ellipse
          cx="100"
          cy="125"
          rx="15"
          ry="10"
          fill="currentColor"
          opacity="0.8"
        />

        {/* Mouth suggestion */}
        <path
          d="M 85 135 Q 100 140 115 135"
          stroke="currentColor"
          stroke-width="3"
          fill="none"
          opacity="0.6"
        />

        {/* Top of head/brow ridge */}
        <ellipse
          cx="100"
          cy="80"
          rx="45"
          ry="20"
          fill="currentColor"
          opacity="0.4"
        />
      </svg>

      {/* Label */}
      <div class="text-lightSlate-600 flex flex-col items-center gap-1 text-center">
        <div class="font-mono text-sm opacity-50">
          <Trans key="ui:_trn_placeholder" />
        </div>
        <div class="text-lightSlate-500 font-medium">{props.variant}</div>
      </div>
    </div>
  )
}
