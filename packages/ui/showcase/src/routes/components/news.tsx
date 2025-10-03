import { createFileRoute } from "@tanstack/solid-router"
import { News } from "../../../../src"
import ComponentDemo from "../../components/ComponentDemo"

export const Route = createFileRoute("/components/news")({
  component: NewsPage
})

function NewsPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">News</h1>
        <p class="text-xl text-gray-600">
          News component for displaying news articles and updates.
        </p>
      </div>

      <ComponentDemo
        title="News Article"
        description="Display news content with title, description, and metadata"
      >
        <News
          slides={[
            {
              title: "GDLauncher 1.2.0 Released",
              description:
                "We're excited to announce the release of GDLauncher 1.2.0 with new features, performance improvements, and bug fixes.",
              date: "2024-01-15",
              image: "https://picsum.photos/800/400?random=1"
            }
          ]}
        />
      </ComponentDemo>

      <ComponentDemo
        title="News List"
        description="Multiple news articles in a list format"
      >
        <News
          slides={[
            {
              title: "New Modpack Support Added",
              description:
                "Enhanced support for popular modpacks with automatic dependency resolution.",
              date: "2024-01-10",
              image: "https://picsum.photos/800/400?random=2"
            },
            {
              title: "Performance Improvements",
              description:
                "Faster startup times and reduced memory usage in the latest update.",
              date: "2024-01-05",
              image: "https://picsum.photos/800/400?random=3"
            },
            {
              title: "UI Refresh",
              description:
                "Updated interface with improved accessibility and modern design elements.",
              date: "2024-01-01",
              image: "https://picsum.photos/800/400?random=4"
            }
          ]}
        />
      </ComponentDemo>
    </div>
  )
}
