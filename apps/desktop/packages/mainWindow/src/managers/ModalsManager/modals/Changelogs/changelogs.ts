export interface ChangelogEntry {
  title: string
  description?: string
  media?: string // URL to gif/video for hero features
}

export interface Changelog {
  highlights: ChangelogEntry[] // Main features showcased with alternating layout
  new: ChangelogEntry[]
  fixed: ChangelogEntry[]
  improved: ChangelogEntry[]
}

const changelogs: Changelog = {
  highlights: [
    {
      title: "Instance Sharing",
      description:
        "Share your modpack instances with anyone via a simple link. Recipients can preview what they're importing before adding it to their library. Requires a GDL account with verified email.",
      media: "https://cdn.gdl.gg/launcher/changelog/2.0.31/instance_sharing.h264.mp4"
    },
    {
      title: "Instance Folders & Drag and Drop",
      description:
        "Organize your library with folders. Drag and drop instances freely, use multi-select for batch operations, and pin your favorites for quick access.",
      media: "https://cdn.gdl.gg/launcher/changelog/2.0.31/drag_drop.h264.mp4"
    },
    {
      title: "Server Management",
      description:
        "Add and manage Minecraft servers directly from GDLauncher. View the live console, manage server properties, track connected players, and install addons.",
      media: "https://cdn.gdl.gg/launcher/changelog/2.0.31/servers.h264.mp4"
    }
  ],
  new: [
    {
      title: "Collapsible Search Filter Sidebar",
      description:
        "A new collapsible sidebar replaces the old filters dropdown, making it easier to filter by platform, game version, modloader, categories, and environment."
    },
    {
      title: "Pre-launch Memory Check",
      description:
        "A warning modal now appears before launching if your system doesn't have enough available memory."
    },
    {
      title: "Cache Cleanup",
      description:
        "A new cache cleanup tool lets you reclaim disk space by clearing mod, modpack, and Minecraft asset caches on demand."
    },
    {
      title: "Cancel Account Deletion",
      description:
        "Scheduled GDL account deletions can now be cancelled from the account settings before they complete."
    },
    {
      title: "GDL Account Status Display",
      description:
        "Ban and unavailable account states are now clearly shown in the UI."
    },
    {
      title: "Email Verification Required",
      description:
        "Features requiring a verified email now show a clear placeholder instead of silently failing."
    },
    {
      title: "Third-Party Licenses",
      description:
        "Added third-party license attribution for all open-source dependencies."
    }
  ],
  fixed: [
    {
      title: "Fixed GDL account creation from Settings"
    },
    {
      title: "Fixed library not updating correctly in some cases"
    },
    {
      title: "Fixed GDL account error messages not showing properly"
    },
    {
      title: "Fixed a deadlock when rearranging library groups"
    },
    {
      title: "Fixed view transition artifacts when switching library modes"
    }
  ],
  improved: [
    {
      title: "Smoother search experience",
      description:
        "Search is now debounced with skeleton loading placeholders for a more responsive feel."
    },
    {
      title: "Better slider controls",
      description:
        "Sliders now show formatted tooltips and have improved drag behavior."
    },
    {
      title: "More reliable GDL session",
      description:
        "GDL tokens are now refreshed on every startup to keep your session valid."
    },
    {
      title: "Polished library animations",
      description:
        "Instance tile animations have been unified for a smoother library experience, with autoscroll while dragging and loading spinners on favorited instances."
    },
    {
      title: "Refined share and import dialogs",
      description:
        "Clearer steps, better error handling, and importing a shared instance now takes you straight to your library."
    },
    {
      title: "Clearer server task progress",
      description:
        "Server install and launch tasks now surface richer progress information and logging."
    },
    {
      title: "Updated dependencies",
      description:
        "Updated Node.js and all project dependencies for better performance and security."
    },
    {
      title: "Improved internal communication",
      description:
        "Switched to a more reliable transport layer between the frontend and backend."
    }
  ]
}

export default changelogs
