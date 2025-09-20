import { For } from "solid-js"
import { Skeleton } from "../Skeleton"

interface AuthorsSkeletonProps {
  count?: number
  size?: "sm" | "md" | "lg"
}

export const AuthorsSkeleton = (props: AuthorsSkeletonProps) => {
  const count = () => props.count ?? 3
  const size = () => props.size ?? "md"

  const sizeClasses = () => {
    switch (size()) {
      case "sm":
        return {
          avatar: "w-6 h-6",
          overlap: "-ml-1",
          icon: "w-4 h-4"
        }
      case "lg":
        return {
          avatar: "w-8 h-8",
          overlap: "-ml-2",
          icon: "w-5 h-5"
        }
      default: // md
        return {
          avatar: "w-7 h-7",
          overlap: "-ml-1.5",
          icon: "w-5 h-5"
        }
    }
  }

  return (
    <div class="flex items-center">
      <Skeleton class={`${sizeClasses().icon} mr-2 rounded`} />
      <div class="flex items-center">
        <For each={Array.from({ length: count() })}>
          {(_, index) => (
            <div class="relative">
              {/* Skeleton cutout ring */}
              <div
                class="absolute rounded-full border-4 border-darkSlate-800"
                style={{
                  width: `calc(${sizeClasses().avatar.split(" ")[0].replace("w-", "")} * 0.25rem + 8px)`,
                  height: `calc(${sizeClasses().avatar.split(" ")[1].replace("h-", "")} * 0.25rem + 8px)`,
                  left: "-4px",
                  top: "-4px",
                  "z-index": "-1"
                }}
              />
              <Skeleton
                class={`${sizeClasses().avatar} ${index() > 0 ? sizeClasses().overlap : ""} rounded-full relative z-10`}
                style={{ "z-index": `${10 + index()}` }}
              />
            </div>
          )}
        </For>
      </div>
    </div>
  )
}

export default AuthorsSkeleton
