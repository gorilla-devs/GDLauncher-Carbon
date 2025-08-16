import { createFileRoute } from "@tanstack/solid-router"
import { Skeleton } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/skeleton")({
  component: SkeletonPage
})

function SkeletonPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Skeleton
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Loading placeholder that mimics the structure of content while it
          loads.
        </p>
      </div>

      <ComponentDemo
        title="Basic Skeleton"
        description="Simple skeleton loading placeholders"
      >
        <div class="space-y-3">
          <Skeleton style={{ width: "100%", height: "20px" }} />
          <Skeleton style={{ width: "75%", height: "20px" }} />
          <Skeleton style={{ width: "50%", height: "20px" }} />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Card Skeleton"
        description="Skeleton for card-like content"
      >
        <div class="space-y-3">
          <Skeleton style={{ width: "100%", height: "200px" }} />
          <Skeleton style={{ width: "100%", height: "24px" }} />
          <Skeleton style={{ width: "80%", height: "16px" }} />
          <Skeleton style={{ width: "60%", height: "16px" }} />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Profile Skeleton"
        description="Skeleton for user profile content"
      >
        <div class="flex space-x-4">
          <Skeleton
            class="rounded-full"
            style={{ width: "60px", height: "60px" }}
          />
          <div class="space-y-2 flex-1">
            <Skeleton style={{ width: "40%", height: "20px" }} />
            <Skeleton style={{ width: "60%", height: "16px" }} />
            <Skeleton style={{ width: "80%", height: "16px" }} />
          </div>
        </div>
      </ComponentDemo>
    </div>
  )
}
