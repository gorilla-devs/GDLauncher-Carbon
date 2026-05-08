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
      media:
        "https://cdn.gdl.gg/launcher/changelog/2.0.31/instance_sharing.h264.mp4"
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
      title: "Modpack Reinstall",
      description:
        "Reinstall instances and servers from their original modpack with one click. Mods, libraries, and the modloader are wiped and redownloaded, while worlds, save data, and key config files (server.properties, ops, whitelist, etc.) are preserved."
    },
    {
      title: "Shader Installation Wizard",
      description:
        "Installing a shader pack now walks you through any setup the instance needs — the wizard auto-installs Fabric plus Iris (or Oculus on Forge) when missing, then adds the shader."
    },
    {
      title: "Dismiss Memory Warning",
      description:
        "The 'Insufficient Memory' prompt before launch can now be permanently dismissed; the toggle lives in Java settings."
    },
    {
      title: "Post-Mortem Server Logs",
      description:
        "View the last server session's logs after the server crashes or is stopped, instead of the console going blank as soon as the process exits."
    },
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
    },
    {
      title: "Switch to an Existing Data Folder",
      description:
        "When you point GDLauncher at a folder that already contains a runtime data set, you can now switch to it as-is — no files are copied or removed."
    }
  ],
  fixed: [
    {
      title:
        "Fixed deep-link URLs (gdlauncher://, curseforge://, modrinth://) not opening on cold start",
      description:
        "Links arriving while the launcher wasn't already running used to be dropped on Windows and Linux."
    },
    {
      title:
        "Fixed the search tab not tracking the active project type when arriving from an instance or server",
      description:
        "Clicking 'Add addons' would sometimes leave the tab unselected or stuck on the previous session's modpack tab."
    },
    {
      title: "Fixed an i18n runtime warning about html-parse-string"
    },
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
    },
    {
      title:
        "Fixed runtime path migration leaving copied files in the target folder if it failed partway through",
      description:
        "If migration aborts now, only files actually created by the migration are rolled back — pre-existing files in the target folder are left untouched."
    },
    {
      title:
        "Fixed servers being launchable while install, stop, or delete operations are in progress"
    },
    {
      title:
        "Fixed timers and listeners not being cleaned up in some dropdowns and search components",
      description:
        "Several search inputs, multi-selects, and instance dropdowns held onto pending timers or socket connections after closing or navigating away."
    },
    {
      title:
        "Fixed browser-style back/forward navigation losing modal state when closing a non-topmost modal"
    },
    {
      title:
        "Fixed instance deletion errors silently accumulating internal state over a long session"
    }
  ],
  improved: [
    {
      title: "Simpler cache cleanup",
      description:
        "The cleanup dialog is now organized into two clear options — GDLauncher cache and Minecraft cache — with smoother progress reporting while it runs."
    },
    {
      title: "Smarter addon search defaults",
      description:
        "When you click 'Add addons', the search now defaults to mods or shaders based on whether your instance has a modloader."
    },
    {
      title: "Sortable addon columns",
      description:
        "Platform and update-available columns in the addon table are now sortable."
    },
    {
      title: "Safer server reinstalls",
      description:
        "Server worlds, dimension folders, and key config files (eula, ops, whitelist, banned-player lists) are now protected end-to-end during install and reinstall, even from malformed modpacks that try to ship them."
    },
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
    },
    {
      title: "Security hardening",
      description:
        "The local API now requires per-session authentication so other applications on your machine can no longer issue commands to GDLauncher. Microsoft sign-in uses PKCE and state validation per RFC 8252. Mod downloads verify stronger checksums (SHA-512 from Modrinth, SHA-1 from CurseForge). Modpack archive extraction blocks path-traversal attacks, including malicious symlinks and absolute paths. External link handling is restricted to http, https, and mailto schemes. OAuth codes and API tokens are redacted in logs. Local database files and the API token file are created with restricted permissions on Linux and macOS. DevTools is disabled on packaged builds."
    }
  ]
}

export default changelogs
