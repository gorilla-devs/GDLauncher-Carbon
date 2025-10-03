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
  new: [
    {
      title: "New Themes",
      description:
        "Added three new themes: Inferno (fiery crimson theme), Aether (ethereal void theme with purples), and Frost (icy arctic theme with cool blues)."
    },
    {
      title: "Unified Search & Discovery System",
      description:
        "Complete overhaul replacing separate mod and modpack pages with a unified search experience supporting all addon types."
    },
    {
      title: "Multiple Addon Type Support",
      description:
        "Expanded beyond mods and modpacks to support shaders, resource packs, data packs, and worlds with type-aware installation and management."
    },
    {
      title: "Advanced Search Filters",
      description:
        "Enhanced filtering system with visual filter badges, platform-specific filters, game version filtering, modloader filtering, category filtering, and environment filtering."
    },
    {
      title: "Redesigned Instance Addons Management",
      description:
        "New unified Addons tab replacing the separate Mods tab, featuring a comprehensive addon table with bulk operations, multi-select support, and enhanced addon information display."
    },
    {
      title: "Logs Finder",
      description:
        "New logs finder functionality for easier log file navigation and analysis with search and filtering capabilities."
    },
    {
      title: "News & Updates System",
      description:
        "Dedicated news page with latest Minecraft updates and patch notes."
    },
    {
      title: "Author Avatars",
      description: "now displayed throughout the interface."
    },
    {
      title: "Enhanced Addon Details",
      description:
        "Improved addon detail pages with better metadata display, author information, version tracking, and platform integration."
    },
    {
      title: "Bulk Addon Operations",
      description:
        "Multi-select support in addon tables with bulk update, enable, disable, and delete operations."
    }
  ],
  fixed: [
    {
      title: "Fixed authentication issues with GDL accounts.",
      description: "The token is now refreshed before opening the app window."
    },
    {
      title: "Removed LWJGL debug mode.",
      description:
        "It was causing issues with some mods (e.g. CustomLoadingScreen)."
    },
    {
      title: "Fixed macOS Sequoia (15.0) display crashes",
      description:
        "Resolved display-info crashes that were occurring on macOS Sequoia (version 15.0)."
    },
    {
      title: "Fixed expired account detection"
    },
    {
      title: "Enhanced mod metadata parsing",
      description:
        "Fixed edge cases in Fabric and NeoForge mod metadata parsing with control character sanitization."
    },
    {
      title: "Fixed search navigation loops",
      description:
        "Prevented duplicate search page navigation and improved search flow."
    },
    {
      title: "Fixed virtual scrolling in addon lists",
      description: "Resolved issues with virtual items in addon list displays."
    },
    {
      title: "Fixed loading states for mod installations",
      description:
        "Improved loading state handling during mod installation processes."
    },
    {
      title:
        "Fixed caching errors that were causing the cache subroutine to hang/crash"
    }
  ],
  improved: [
    {
      title: "Complete UI Component System Overhaul",
      description:
        "Redesigned notification system, enhanced select components, new progress indicators, improved skeleton loading patterns, and better popover positioning."
    },
    {
      title: "Better Platform Integration",
      description:
        "Improved integration between CurseForge and Modrinth with unified addon linking and cross-platform compatibility."
    },
    {
      title: "Improved Addon Table Experience",
      description:
        "Enhanced addon tables with sortable columns, better information display, right-click context menus, and improved selection handling."
    },
    {
      title: "Enhanced NeoForge Support",
      description:
        "Improved support for NeoForge mods with better metadata detection and parsing."
    },
    {
      title: "Better Installation Flow",
      description:
        "Streamlined addon installation process with improved instance selection and better progress tracking."
    },
    {
      title: "Updated Electron and dependencies",
      description:
        "Updated to Electron 37.2.6 and Node.js 22.12.0 for better performance and security."
    }
  ]
}

export default changelogs
