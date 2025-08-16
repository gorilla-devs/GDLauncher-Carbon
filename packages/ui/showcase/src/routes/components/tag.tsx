import { createFileRoute } from "@tanstack/solid-router"
import { Tag } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/tag")({
  component: TagPage
})

function TagPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1
          class="text-4xl font-bold mb-4"
          style={`color: rgb(var(--lightSlate-50))`}
        >
          Tag
        </h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Compact label component for categorizing, filtering, or displaying
          metadata.
        </p>
      </div>

      <ComponentDemo
        title="Basic Tags"
        description="Simple tags with different content"
      >
        <div class="flex flex-wrap gap-2">
          <Tag name="JavaScript" />
          <Tag name="React" />
          <Tag name="SolidJS" />
          <Tag name="TypeScript" />
          <Tag name="Vite" />
        </div>
      </ComponentDemo>

      <ComponentDemo title="Tag Sizes" description="Different tag sizes">
        <div class="flex flex-wrap gap-2 items-center">
          <Tag name="Small Tag" size="small" />
          <Tag name="Medium Tag" size="medium" />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Tags with Images"
        description="Tags can include icons or images"
      >
        <div class="flex flex-wrap gap-2">
          <Tag
            name="JavaScript"
            img="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/javascript/javascript-original.svg"
          />
          <Tag
            name="React"
            img="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/react/react-original.svg"
          />
          <Tag
            name="TypeScript"
            img="https://cdn.jsdelivr.net/gh/devicons/devicon/icons/typescript/typescript-original.svg"
          />
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Fixed vs Default Tags"
        description="Different tag types - fixed tags cannot be removed"
      >
        <div class="flex flex-wrap gap-2">
          <Tag
            name="Removable Tag"
            type="default"
            onClose={(name) => alert(`Removed: ${name}`)}
          />
          <Tag name="Fixed Tag" type="fixed" />
          <Tag name="Another Removable" type="default" />
        </div>
      </ComponentDemo>
    </div>
  )
}
