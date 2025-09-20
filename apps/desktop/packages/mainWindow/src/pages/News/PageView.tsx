import { Show, createMemo } from "solid-js"
import { useParams } from "@solidjs/router"
import { useNews, usePatchNotes } from "@/utils/news"
import { Button } from "@gd/ui"
import { useGDNavigate } from "@/managers/NavigationManager"
import { parseToHtml } from "@/utils/modplatformDescriptionConverter"
import { createQuery } from "@tanstack/solid-query"

const PageView = () => {
  const params = useParams()
  const navigator = useGDNavigate()
  const news = useNews()
  const patchNotes = usePatchNotes()

  const currentArticle = createMemo(() => {
    // Wait for data to be loaded
    if (news.isPending || patchNotes.isPending) return undefined

    // First try to find in news
    const newsItem = news.data?.find((item) => item.id === params.id)
    if (newsItem) return newsItem

    // Then try to find in patch notes
    const patchItem = patchNotes.data?.find((item) => item.id === params.id)
    return patchItem
  })

  // Fetch full patch content if it's a patch note
  const fetchPatchContent = async (contentPath: string) => {
    try {
      const resp = await fetch(contentPath)
      const data = await resp.json()
      return data.body || ""
    } catch (err) {
      console.error("Error fetching patch content:", err)
      return ""
    }
  }

  const patchContent = createQuery(() => ({
    queryKey: ["patchContent", currentArticle()?.contentPath],
    queryFn: () => fetchPatchContent(currentArticle()!.contentPath!),
    enabled:
      !!currentArticle()?.contentPath && currentArticle()?.type === "patch",
    staleTime: 30 * 60 * 1000 // 30 minutes
  }))

  return (
    <div class="max-w-4xl mx-auto p-6">
      <Show
        when={!news.isPending && !patchNotes.isPending && currentArticle()}
        fallback={
          <div class="text-center py-20">
            <div class="bg-darkSlate-700 rounded-2xl p-12 border border-darkSlate-600">
              <Show
                when={news.isPending || patchNotes.isPending}
                fallback={
                  <>
                    <h1 class="text-3xl font-bold mb-6 text-white">
                      Article not found
                    </h1>
                    <p class="text-lightSlate-400 mb-8">
                      The article you're looking for doesn't exist or has been
                      removed.
                    </p>
                    <Button
                      onClick={() => navigator.navigate("/news")}
                      class="px-8 py-3"
                    >
                      Back to News
                    </Button>
                  </>
                }
              >
                <div class="flex items-center justify-center gap-3 text-lightSlate-400">
                  <div class="animate-spin i-ri:loader-4-line text-2xl" />
                  <span class="text-xl">Loading article...</span>
                </div>
              </Show>
            </div>
          </div>
        }
      >
        {(article) => (
          <article class="flex flex-col gap-8">
            <style>
              {`
                  #news_article_content *::selection {
                    background: rgb(var(--primary-500) / 0.3);
                    color: rgb(var(--lightSlate-50));
                  }
                `}
            </style>
            {/* Navigation */}
            <button
              onClick={() => navigator.navigate("/news")}
              class="self-start text-lightSlate-400 hover:text-lightSlate-200 flex items-center gap-3 mb-2 transition-colors group"
            >
              <div class="i-ri:arrow-left-line group-hover:transform group-hover:-translate-x-1 transition-transform" />
              <span class="font-medium">Back to News</span>
            </button>

            {/* Hero Image */}
            <div class="relative overflow-hidden rounded-2xl shadow-2xl">
              <img
                src={article().image}
                alt={article().title}
                class="w-full h-96 object-cover"
              />
              <div class="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent" />
            </div>

            {/* Header */}
            <header class="flex flex-col gap-6 bg-darkSlate-700 rounded-2xl p-8 border border-darkSlate-600">
              <div class="flex items-start justify-between gap-4">
                <h1 class="text-5xl font-bold leading-tight text-white">
                  {article().title}
                </h1>
                {article().type === "patch" && (
                  <div class="flex flex-col gap-2 items-end">
                    <span class="text-sm bg-primary-600/20 text-primary-300 px-4 py-2 rounded-full border border-primary-600/30 font-semibold">
                      Patch Notes
                    </span>
                    {article().version && (
                      <span class="text-xs text-lightSlate-400 font-mono">
                        v{article().version}
                      </span>
                    )}
                  </div>
                )}
              </div>

              <div class="flex items-center gap-4 text-lightSlate-300">
                <div class="flex items-center gap-2">
                  <i class="i-ri:calendar-line text-primary-400" />
                  <time class="text-lg font-medium">
                    {new Date(article().date).toLocaleDateString(undefined, {
                      year: "numeric",
                      month: "long",
                      day: "numeric"
                    })}
                  </time>
                </div>
                <div class="w-1 h-1 bg-lightSlate-500 rounded-full" />
                <span class="text-sm capitalize font-medium text-lightSlate-400">
                  {article().type === "patch"
                    ? "Minecraft Patch"
                    : "Minecraft News"}
                </span>
              </div>
            </header>

            {/* Content */}
            <div
              id="news_article_content"
              class="bg-darkSlate-700 rounded-2xl p-8 border border-darkSlate-600"
            >
              {article().type === "patch" ? (
                <Show
                  when={patchContent.data}
                  fallback={
                    <div class="flex items-center justify-center py-16">
                      <div class="flex items-center gap-3 text-lightSlate-400">
                        <div class="animate-spin i-ri:loader-4-line text-xl" />
                        <span class="text-lg">Loading patch content...</span>
                      </div>
                    </div>
                  }
                >
                  <div class="patch-content">
                    <style>
                      {`
                          .patch-content h1, .patch-content h2, .patch-content h3, .patch-content h4, .patch-content h5, .patch-content h6 {
                            font-weight: 700;
                            margin: 1.5rem 0 1rem 0;
                          }
                          .patch-content h1 {
                            font-size: 2rem;
                            color: rgb(var(--primary-400));
                            border-bottom: 2px solid rgb(var(--primary-500));
                            padding-bottom: 0.5rem;
                          }
                          .patch-content h2 {
                            font-size: 1.5rem;
                            color: rgb(var(--primary-400));
                          }
                          .patch-content h3 {
                            font-size: 1.25rem;
                            color: rgb(var(--primary-300));
                          }
                          .patch-content h4 {
                            color: rgb(var(--primary-300));
                          }
                          .patch-content h5 {
                            color: rgb(var(--primary-200));
                          }
                          .patch-content h6 {
                            color: rgb(var(--primary-200));
                          }
                          .patch-content p {
                            color: rgb(var(--lightSlate-300));
                            line-height: 1.7;
                            margin: 1rem 0;
                          }
                          .patch-content ul, .patch-content ol {
                            color: rgb(var(--lightSlate-300));
                            margin: 1rem 0;
                            padding-left: 1.5rem;
                          }
                          .patch-content li {
                            margin: 0.5rem 0;
                            line-height: 1.6;
                          }
                          .patch-content strong {
                            color: rgb(var(--lightSlate-50));
                            font-weight: 600;
                          }
                          .patch-content em {
                            color: rgb(var(--lightSlate-400));
                            font-style: italic;
                          }
                          .patch-content code {
                            background: rgb(var(--darkSlate-700));
                            color: rgb(var(--primary-300));
                            padding: 0.2rem 0.4rem;
                            border-radius: 0.25rem;
                            font-family: 'JetBrains Mono', monospace;
                            font-size: 0.9em;
                          }
                          .patch-content pre {
                            background: rgb(var(--darkSlate-900));
                            color: rgb(var(--lightSlate-200));
                            padding: 1rem;
                            border-radius: 0.5rem;
                            overflow-x: auto;
                            margin: 1rem 0;
                            border: 1px solid rgb(var(--darkSlate-600));
                          }
                          .patch-content a {
                            color: rgb(var(--primary-400));
                            text-decoration: underline;
                            transition: color 0.2s;
                          }
                          .patch-content a:hover {
                            color: rgb(var(--primary-300));
                          }
                          .patch-content img {
                            max-width: 100%;
                            height: auto;
                            border-radius: 0.5rem;
                            margin: 1rem 0;
                            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
                          }
                          .patch-content blockquote {
                            border-left: 4px solid rgb(var(--primary-500));
                            padding-left: 1rem;
                            margin: 1rem 0;
                            background: rgba(var(--primary-500), 0.1);
                            border-radius: 0 0.25rem 0.25rem 0;
                            padding: 1rem;
                          }
                          .patch-content table {
                            width: 100%;
                            border-collapse: collapse;
                            margin: 1rem 0;
                            background: rgb(var(--darkSlate-700));
                            border-radius: 0.5rem;
                            overflow: hidden;
                          }
                          .patch-content th, .patch-content td {
                            padding: 0.75rem;
                            text-align: left;
                            border-bottom: 1px solid rgb(var(--darkSlate-600));
                          }
                          .patch-content th {
                            background: rgb(var(--darkSlate-900));
                            color: rgb(var(--lightSlate-100));
                            font-weight: 600;
                          }
                        `}
                    </style>
                    <div
                      class="text-lightSlate-100 leading-relaxed prose prose-lg prose-invert max-w-none"
                      // eslint-disable-next-line solid/no-innerhtml
                      innerHTML={parseToHtml(patchContent.data)}
                    />
                  </div>
                </Show>
              ) : (
                <div class="prose prose-xl prose-invert max-w-none">
                  <p class="text-lightSlate-100 leading-relaxed">
                    {article().description}
                  </p>
                </div>
              )}
            </div>

            {/* Action Button */}
            <div class="flex justify-center pt-4">
              <Button
                onClick={() => window.openExternalLink(article().url)}
                class="flex items-center gap-3 px-8 py-4 text-lg font-semibold bg-gradient-to-r from-primary-600 to-primary-700 hover:from-primary-500 hover:to-primary-600 border border-primary-500/50 shadow-lg hover:shadow-primary-500/25 transition-all duration-300"
              >
                <span>
                  {article().type === "patch"
                    ? "Read Full Patch Notes"
                    : "Read Full Article"}
                </span>
                <div class="i-ri:external-link-line text-xl" />
              </Button>
            </div>
          </article>
        )}
      </Show>
    </div>
  )
}

export default PageView
