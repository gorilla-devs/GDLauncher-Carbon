export interface ChangelogEntry {
  title: string
  description?: string
}

export interface Changelog {
  new: ChangelogEntry[]
  fixed: ChangelogEntry[]
  improved: ChangelogEntry[]
}

const changelogs: Changelog = {
  new: [],
  fixed: [
    {
      title: "Fixed authentication issues with GDL accounts.",
      description: "The token is now refreshed before opening the app window."
    },
    {
      title: "Removed LWJGL debug mode.",
      description:
        "It was causing issues with some mods (e.g. CustomLoadingScreen)."
    }
  ],
  improved: []
}

export default changelogs
