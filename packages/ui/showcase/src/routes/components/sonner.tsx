import { createFileRoute } from "@tanstack/solid-router"
import { Toaster } from "../../../../src"
import { toast } from "somoto"
import { Button } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/sonner")({
  component: SonnerPage
})

function SonnerPage() {
  return (
    <div class="max-w-4xl">
      <Toaster />
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style="color: rgb(var(--lightSlate-50))"
        >
          Sonner
        </h1>
        <p class="text-xl" style="color: rgb(var(--lightSlate-300))">
          An opinionated toast component for Solid with beautiful animations,
          custom icons, and rich features that matches your UI theme.
        </p>
      </div>

      <ComponentDemo
        title="Basic Toast"
        description="Simple toast notification"
      >
        <Button type="primary" onClick={() => toast("Hello World!")}>
          Show Toast
        </Button>
      </ComponentDemo>

      <ComponentDemo
        title="Toast with Description"
        description="Toast with additional description text"
      >
        <Button
          type="secondary"
          onClick={() =>
            toast("Event has been created", {
              description: "Sunday, December 03, 2023 at 9:00 AM"
            })
          }
        >
          Show Toast with Description
        </Button>
      </ComponentDemo>

      <ComponentDemo
        title="Toast with Action"
        description="Toast with action button"
      >
        <Button
          type="outline"
          onClick={() =>
            toast("Event has been created", {
              description: "Sunday, December 03, 2023 at 9:00 AM",
              action: {
                label: "Undo",
                onClick: () => console.log("Undo")
              }
            })
          }
        >
          Show Toast
        </Button>
      </ComponentDemo>

      <ComponentDemo
        title="Different Toast Types with Custom Icons"
        description="Various toast types with color-coded icons and styling"
      >
        <div class="flex flex-wrap gap-4">
          <Button
            type="primary"
            onClick={() =>
              toast.success("Operation completed successfully!", {
                description: "Your changes have been saved"
              })
            }
          >
            Success Toast
          </Button>
          <Button
            type="secondary"
            onClick={() =>
              toast.error("Something went wrong!", {
                description: "Please try again later"
              })
            }
          >
            Error Toast
          </Button>
          <Button
            type="outline"
            onClick={() =>
              toast.warning("This is a warning!", {
                description: "Please review your changes"
              })
            }
          >
            Warning Toast
          </Button>
          <Button
            type="transparent"
            onClick={() =>
              toast.info("Here's some helpful info", {
                description: "This feature has been updated"
              })
            }
          >
            Info Toast
          </Button>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Toast with Custom Duration"
        description="Control how long the toast stays visible"
      >
        <div class="flex flex-wrap gap-4">
          <Button
            type="primary"
            onClick={() =>
              toast("Quick toast", {
                duration: 1000,
                description: "This will disappear quickly"
              })
            }
          >
            1 Second
          </Button>
          <Button
            type="secondary"
            onClick={() =>
              toast("Long toast", {
                duration: 10000,
                description: "This will stay for a while"
              })
            }
          >
            10 Seconds
          </Button>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Advanced Toast Examples"
        description="Loading states and promise-based toasts"
      >
        <div class="flex flex-wrap gap-4">
          <Button
            type="glow"
            onClick={() => {
              const promise = new Promise((resolve) =>
                setTimeout(() => resolve("Data loaded!"), 2000)
              )
              toast.promise(promise, {
                loading: "Loading data...",
                success: "Data loaded successfully!",
                error: "Failed to load data"
              })
            }}
          >
            Promise Toast
          </Button>
          <Button
            type="glass"
            onClick={() =>
              toast.loading("Processing...", {
                description: "Please wait while we process your request"
              })
            }
          >
            Loading Toast
          </Button>
        </div>
      </ComponentDemo>
    </div>
  )
}
