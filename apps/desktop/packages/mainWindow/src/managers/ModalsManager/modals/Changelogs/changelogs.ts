export interface ChangelogEntry {
  title: string
  description?: string
  media?: string // URL to gif/video for hero features
}

export interface Changelog {
  new: ChangelogEntry[]
  fixed: ChangelogEntry[]
  improved: ChangelogEntry[]
}

const changelogs: Changelog = {
  new: [
    {
      title: "Redesigned Addons Browser",
      description:
        "Completely rethought and redesigned addons browser with support for mods, resource packs, shaders, modpacks, data packs, and worlds. Features a unified search experience, enhanced filtering, type-aware installation, and a modern interface that replaces the old separate mod and modpack pages.",
      media:
        "https://cdn.gdl.gg/launcher/changelog/2.0.26/addons-browser-overhaul.mp4"
    },
    {
      title: "Addons Grid View Mode",
      description:
        "New grid view mode for browsing addons with a visual card-based layout as an alternative to the list view."
    },
    {
      title: "'New' Badge for Recently Installed Instances",
      description:
        "Newly installed instances now display a 'New' badge, making it easier to identify your latest additions."
    },
    {
      title: "Log File Rotation & Cleanup",
      description:
        "Automatic log file rotation and cleanup to prevent log files from growing too large, along with temporary file cleanup on startup."
    },
    {
      title: "Release Channel Branding",
      description:
        "Alpha and beta versions now display distinct logos to clearly indicate which release channel you're using."
    },
    {
      title: "Redesigned Authentication Flow",
      description:
        "Completely redesigned authentication experience with a new welcome screen, separate terms and privacy step, QR code support for device code authentication, and clearer step-by-step progression through the sign-in process."
    },
    {
      title: "Duplicated Mods Resolution Wizard",
      description:
        "New multi-step wizard automatically detects and helps resolve duplicate mods in your instances. Choose to disable or delete duplicate versions with a comprehensive summary before applying changes."
    },
    {
      title: "New Themes",
      description:
        "Added three new themes: Inferno (fiery crimson theme), Aether (ethereal void theme with purples), and Frost (icy arctic theme with cool blues)."
    },
    {
      title: "Advanced Search Filters",
      description:
        "Enhanced filtering system with visual filter badges, platform-specific filters, game version filtering, modloader filtering, category filtering, and environment filtering."
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
    },
    {
      title: "Version Type Filtering in Patch Notes",
      description:
        "Filter patch notes by release type (release, snapshot, etc.) to easily find the updates you're looking for."
    }
  ],
  fixed: [
    {
      title: "Fixed navbar crash",
      description:
        "Resolved a crash that could occur in the navigation bar under certain conditions."
    },
    {
      title: "Fixed instance export modal",
      description:
        "Instance export now properly receives and uses the correct instance ID."
    },
    {
      title: "Fixed modpack version changes",
      description:
        "Resolved issues with empty versions when navigating from instance to modpack and improved Modrinth modpack latest version selection."
    },
    {
      title: "Fixed authentication issues with GDL accounts",
      description: "The token is now refreshed before opening the app window."
    },
    {
      title: "Removed LWJGL debug mode",
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
    },
    {
      title: "Fixed GDL account verification sometimes hanging"
    },
    {
      title: "Fixed mods deletion on version/modloader update",
      description:
        "Resolved an issue where mods could be incorrectly deleted when changing the game version or modloader."
    },
    {
      title: "Fixed account state synchronization",
      description:
        "Account state now properly syncs after enrollment, ensuring your account status is always up to date."
    },
    {
      title: "Fixed game log active state updates",
      description:
        "Game logs now correctly update their active state before cache invalidation."
    }
  ],
  improved: [
    {
      title: "Redesigned Instance Creation Modal",
      description:
        "Improved interface for creating new instances with a cleaner, more intuitive design and better user experience."
    },
    {
      title: "Fully Internationalized Authentication",
      description:
        "All authentication flow text is now fully translatable, including welcome messages, terms and privacy notices, error messages, and helper text, making the sign-in experience accessible to users worldwide."
    },
    {
      title: "Instance Addons Management Overhaul",
      description:
        "Complete redesign of the instance addons management system. New unified Addons tab replaces the old Mods tab, featuring a comprehensive addon table with bulk operations, multi-select support, powerful search and filtering within your installed addons, sortable columns, right-click context menus, and enhanced addon information display with better version tracking."
    },
    {
      title: "Easier addon version changes",
      description:
        "Changing versions of installed addons is now much simpler, especially for mods available on multiple platforms."
    },
    {
      title: "Refreshed icon set",
      description:
        "Updated all icons across the interface for better consistency and visual clarity."
    },
    {
      title: "Enhanced addon filters",
      description:
        "Improved addon type filtering that intelligently shows only compatible types based on your instance configuration (e.g., mods only show when modloaders are present)."
    },
    {
      title: "Better error messages",
      description:
        "Error messages throughout the application are now clearer and more helpful for troubleshooting issues."
    },
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
        "Updated to Electron 37.7.0 and Node.js 22.12.0 for better performance and security."
    },
    {
      title: "Improved auto update system",
      description:
        "Enhanced update installation flow with better error handling, improved download UI, and reliable update installation on quit."
    },
    {
      title: "Better loading indicators",
      description:
        "Improved loading animations throughout the app with more consistent and informative skeleton screens that show the structure of content before it loads."
    },
    {
      title: "Smoother animations",
      description:
        "Enhanced transitions with spring easing physics for more natural-feeling animations throughout the app, including searchbar and page transitions."
    },
    {
      title: "Enhanced Addon View & Install Experience",
      description:
        "Improved addon detail pages with better instance selection and clearer mod installation workflow."
    },
    {
      title: "Better Default Instance Sorting",
      description:
        "Instances now sort by creation date (newest first) by default, making your latest instances easier to find."
    },
    {
      title: "Improved Logs Tab Sidebar",
      description:
        "Reorganized logs tab with better component organization for easier log file navigation."
    },
    {
      title: "Clearer Update Instructions for Package Manager Users",
      description:
        "Manual update messages now mention package manager as an update option for users who installed via package managers."
    },
    {
      title: "Redesigned Tabs Component",
      description:
        "Tabs throughout the app have been redesigned with improved styling and better accessibility using modern UI primitives."
    }
  ]
}

export default changelogs
