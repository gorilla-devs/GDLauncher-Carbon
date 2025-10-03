import { MOJANG_API, NEWS_URL, JAVA_PATCH_NOTES_URL } from "@/constants"
import { createQuery } from "@tanstack/solid-query"

export interface NewsItem {
  title: string
  category: string
  date: string
  text: string
  playPageImage: PlayPageImage
  newsPageImage: NewsPageImage
  readMoreLink: string
  newsType: string[]
  id: string
  tag?: string
  cardBorder?: boolean
  highlight?: Highlight
}

export interface PlayPageImage {
  title: string
  url: string
}

export interface NewsPageImage {
  title: string
  url: string
  dimensions: Dimensions
}

export interface Dimensions {
  width: number
  height: number
}

export interface Highlight {
  image: Image
  iconImage: IconImage
  platforms: string[]
  entitlements: any[]
  title: string
  description: string
  until: string
  playGame?: string
  readMoreLink?: string
}

export interface Image {
  url: string
  title: string
}

export interface IconImage {}

export interface PatchNoteItem {
  id: string
  title: string
  version: string
  type: string
  image: PatchNoteImage
  contentPath: string
  date: string
  shortText: string
}

export interface PatchNoteImage {
  url: string
  title: string
}

export interface ContentItem {
  id: string
  title: string
  description: string
  image: string
  url: string
  date: string
  type: "news" | "patch"
  versionType?: string
  version?: string
  contentPath?: string
}

const fetchPatchNotes = async (): Promise<ContentItem[]> => {
  try {
    const resp = await fetch(JAVA_PATCH_NOTES_URL)
    if (!resp.ok) {
      throw new Error(`Failed to fetch patch notes: ${resp.status} ${resp.statusText}`)
    }
    const data = await resp.json()

    const patchNotesArr =
      data.entries?.map((patchNote: PatchNoteItem) => ({
        id: patchNote.id,
        title: patchNote.title,
        description: patchNote.shortText,
        image: `${MOJANG_API}${patchNote.image.url}`,
        url: `${MOJANG_API}${patchNote.contentPath}`,
        date: patchNote.date,
        type: "patch",
        versionType: patchNote.type,
        version: patchNote.version,
        contentPath: `${MOJANG_API}/v2/javaPatchNotes/${patchNote.id}.json`
      })) || []

    return patchNotesArr
  } catch (err) {
    console.error("Error fetching patch notes:", err)
    return []
  }
}

const fetchNews = async (): Promise<ContentItem[]> => {
  try {
    const resp = await fetch(NEWS_URL)
    if (!resp.ok) {
      throw new Error(`Failed to fetch news: ${resp.status} ${resp.statusText}`)
    }
    const data = await resp.json()
    const filteredNews = data.entries

    const newsArr =
      filteredNews?.map((newsEntry: NewsItem) => ({
        id: newsEntry.id,
        title: newsEntry.title,
        description: newsEntry.text,
        image: `${MOJANG_API}${newsEntry.newsPageImage.url}`,
        url: newsEntry.readMoreLink,
        date: newsEntry.date,
        type: "news"
      })) || []

    return newsArr.slice(0, 20)
  } catch (err) {
    console.error(err)
    return []
  }
}

export const useNews = () => {
  return createQuery(() => ({
    queryKey: ["news"],
    queryFn: fetchNews,
    staleTime: 15 * 60 * 1000, // 15 minutes
    gcTime: 30 * 60 * 1000 // 30 minutes
  }))
}

export const usePatchNotes = () => {
  return createQuery(() => ({
    queryKey: ["patchNotes"],
    queryFn: fetchPatchNotes,
    staleTime: 15 * 60 * 1000, // 15 minutes
    gcTime: 30 * 60 * 1000 // 30 minutes
  }))
}
