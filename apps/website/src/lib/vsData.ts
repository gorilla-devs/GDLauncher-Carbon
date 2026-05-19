/**
 * /vs/* (launcher comparison) page data, indexed by locale.
 *
 * Each comparison page is built from a `ComparisonData` block: structural
 * fields (title, intro, verdict), a feature-by-feature table, and a list of
 * prose sections. The hub page (/vs) pulls from `ComparisonHubData`.
 *
 * Competitor product names (Prism, MultiMC, etc.) are deliberately not
 * translated. Same goes for ecosystem names: CurseForge, Modrinth, Forge,
 * Fabric, NeoForge, Quilt, Java, Linux.
 */
import type { Locale } from "./i18n"
import { DEFAULT_LOCALE } from "./i18n"

export type ComparisonSlug =
  | "prismlauncher"
  | "curseforge-app"
  | "modrinth-app"
  | "atlauncher"
  | "multimc"
  | "ftb-app"
  | "tlauncher"

export const COMPARISON_SLUGS: readonly ComparisonSlug[] = [
  "prismlauncher",
  "curseforge-app",
  "modrinth-app",
  "atlauncher",
  "multimc",
  "ftb-app",
  "tlauncher",
] as const

/** Stays untranslated across locales (these are product names). */
export const COMPETITOR_NAMES: Record<ComparisonSlug, string> = {
  prismlauncher: "Prism Launcher",
  "curseforge-app": "CurseForge App",
  "modrinth-app": "Modrinth App",
  atlauncher: "ATLauncher",
  multimc: "MultiMC",
  "ftb-app": "FTB App",
  tlauncher: "TLauncher",
}

export const COMPETITOR_SHORT: Record<ComparisonSlug, string> = {
  prismlauncher: "Prism",
  "curseforge-app": "CurseForge App",
  "modrinth-app": "Modrinth App",
  atlauncher: "ATLauncher",
  multimc: "MultiMC",
  "ftb-app": "FTB App",
  tlauncher: "TLauncher",
}

export type ComparisonRow = {
  feature: string
  gdl: string
  competitor: string
  note?: string
}

export type ComparisonSection = {
  heading: string
  paragraphs: string[]
}

export type ComparisonData = {
  /** Used for SEO title + on-page H1 ("GDLauncher vs Prism Launcher"). */
  title: string
  /** SEO meta description. */
  description: string
  /** Lead paragraph under the H1. */
  intro: string
  rows: ComparisonRow[]
  /** Bottom-of-page summary paragraph. */
  verdict: string
  sections: ComparisonSection[]
}

export type ComparisonChrome = {
  /** Breadcrumb label for the /vs hub page. */
  compareBreadcrumb: string
  /** Table header for the feature column. */
  feature: string
  /** Download CTA at the bottom of each comparison page. */
  tryGdl: string
  /** Link back to the /vs hub at the bottom of each page. */
  seeAllComparisons: string
  /** Heading above the verdict paragraph. */
  theVerdict: string
}

export type ComparisonHubData = {
  pageTitle: string
  pageDescription: string
  h1: string
  intro: string
  /** Per-competitor blurb shown on the hub. */
  competitors: Record<ComparisonSlug, { blurb: string }>
}

export type LocaleData = {
  chrome: ComparisonChrome
  hub: ComparisonHubData
  /**
   * Per-slug comparison entries. Locales may omit slugs; missing slugs fall
   * back to the default-locale entry in `getComparison`.
   */
  comparisons: Partial<Record<ComparisonSlug, ComparisonData>>
}

// ---------------------------------------------------------------------------
// English (source of truth)
// ---------------------------------------------------------------------------

const en: LocaleData = {
  chrome: {
    compareBreadcrumb: "Compare",
    feature: "Feature",
    tryGdl: "Try GDLauncher",
    seeAllComparisons: "See all comparisons",
    theVerdict: "The verdict",
  },
  hub: {
    pageTitle:
      "GDLauncher vs Other Minecraft Launchers: Side-by-Side Comparisons",
    pageDescription:
      "Detailed comparisons between GDLauncher and other popular Minecraft launchers: Prism Launcher, CurseForge App, Modrinth App, ATLauncher, MultiMC, FTB App, TLauncher.",
    h1: "How GDLauncher compares",
    intro:
      "Picking a Minecraft launcher? Here's how GDLauncher stacks up against the major alternatives, feature by feature. We're biased, but we put the comparisons in writing so you can decide for yourself.",
    competitors: {
      prismlauncher: {
        blurb:
          "Lightweight, open-source, MultiMC fork. How GDLauncher compares on usability and modpack support.",
      },
      "curseforge-app": {
        blurb:
          "The official CurseForge launcher. Comparing CurseForge integration, Modrinth support, and built-in server management.",
      },
      "modrinth-app": {
        blurb:
          "The Modrinth-only launcher. Where GDLauncher gives you both Modrinth and CurseForge in one place.",
      },
      atlauncher: {
        blurb:
          "Veteran modpack launcher. UI, performance, and platform support side by side.",
      },
      multimc: {
        blurb:
          "The lightweight power-user launcher. Where automation and modpack workflows differ.",
      },
      "ftb-app": {
        blurb:
          "Feed The Beast's own launcher for FTB and CurseForge packs. Where Modrinth, Cloud Instance Sharing, and server management differ.",
      },
      tlauncher: {
        blurb:
          "Launcher that skips Mojang authentication. Why that approach is against the EULA and what you give up using it.",
      },
    },
  },
  comparisons: {
    prismlauncher: {
      title: "GDLauncher vs Prism Launcher",
      description:
        "GDLauncher vs Prism Launcher: detailed comparison of features, modpack support, performance, and UI. Find the right Minecraft launcher for your needs.",
      intro:
        "Prism Launcher is the popular open-source MultiMC fork. GDLauncher is a modern launcher with deep CurseForge and Modrinth integration. Here's how they really compare on the things that matter day-to-day.",
      rows: [
        {
          feature: "CurseForge support",
          gdl: "Yes",
          competitor: "Partial (workaround)",
          note: "When a mod opts out of third-party API access, Prism asks you to download that file manually in a browser",
        },
        { feature: "Modrinth support", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto Java management", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto mod updates", gdl: "Yes", competitor: "No (manual check only)" },
        { feature: "Auto modpack updates", gdl: "Yes", competitor: "No (manual check only)" },
        { feature: "Multi-instance", gdl: "Yes", competitor: "Yes" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Yes (one-click code, mixed CF + MR)",
          competitor: "No (manual export, no mixed CF + MR)",
        },
        { feature: "Server management", gdl: "Yes (built-in)", competitor: "No" },
        { feature: "Modern UI", gdl: "Yes", competitor: "No" },
        {
          feature: "Pays addon authors",
          gdl: "Yes",
          competitor: "No",
        },
        { feature: "Source on GitHub", gdl: "Yes", competitor: "Yes" },
        { feature: "Lightweight (RAM)", gdl: "No", competitor: "Yes" },
      ],
      verdict:
        "Prism is excellent if you want a barebones, lightweight launcher and don't mind doing more work yourself for modpacks. GDLauncher is for players who want one-click installs from CurseForge and Modrinth, Cloud Instance Sharing, and built-in server management without leaving the app. If you're new to modded Minecraft or value polish over minimalism, GDLauncher is the easier path.",
      sections: [
        {
          heading: "Modpack workflow",
          paragraphs: [
            "Prism and GDLauncher both browse and install CurseForge packs from inside the launcher, so the everyday experience is similar. The friction lives at the edges: when a mod author has opted out of third-party API access for their file, Prism asks you to click through each blocked link and download those files manually in a browser. GDLauncher's CurseForge partnership fetches those files directly, so installs stay one-click even when packs include blocked mods.",
            "Modrinth packs work the same in both launchers, browse from inside the app and install in one click.",
          ],
        },
        {
          heading: "UI and discovery",
          paragraphs: [
            "Prism's Qt-based UI is functional but utilitarian; the main view is a list of instances. GDLauncher's UI is built specifically for finding and managing modpacks, with a built-in browser, instance grouping, drag-and-drop reordering, and visual cards. Subjective, but worth a look at screenshots.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "GDLauncher has one-click Cloud Instance Sharing: paste a code, get the exact same setup. Prism has instance export/import via files, which works but isn't quite as friction-free for sharing with friends.",
          ],
        },
      ],
    },
    "curseforge-app": {
      title: "GDLauncher vs CurseForge App",
      description:
        "GDLauncher vs CurseForge App: comparison of features, ads, Modrinth support, and server management. Find the better way to play modded Minecraft.",
      intro:
        "The CurseForge App is the official launcher for CurseForge content. GDLauncher integrates with CurseForge too, adds Modrinth in the same browser, Cloud Instance Sharing across both platforms, and built-in server management. Here's the breakdown.",
      rows: [
        {
          feature: "CurseForge support",
          gdl: "Yes",
          competitor: "Yes (native, it's their app)",
        },
        { feature: "Modrinth support", gdl: "Yes", competitor: "No" },
        { feature: "Auto Java management", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto mod updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Auto modpack updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Multi-instance", gdl: "Yes", competitor: "Yes" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Yes (one-click code, mixed CF + MR)",
          competitor: "Yes (CurseForge only)",
        },
        { feature: "Server management", gdl: "Yes (built-in)", competitor: "No" },
        {
          feature: "Ad-supported in app",
          gdl: "Yes (in-app banner)",
          competitor: "Yes (in-app banner)",
        },
        { feature: "Source on GitHub", gdl: "Yes", competitor: "No" },
        { feature: "Pays addon authors", gdl: "Yes", competitor: "Yes" },
      ],
      verdict:
        "If you only ever install CurseForge content, the CurseForge App is the official choice. GDLauncher gives you the same CurseForge integration plus Modrinth in the same browser, Cloud Instance Sharing that travels with mixed CurseForge + Modrinth setups, and built-in server management.",
      sections: [
        {
          heading: "Modrinth in the same launcher",
          paragraphs: [
            "The CurseForge App is, by design, CurseForge-only. Modrinth has been growing fast, especially for Fabric mods, performance mods, and shaders, and many authors now publish to both platforms. GDLauncher's built-in browser searches both at once, so you don't have to pick.",
          ],
        },
        {
          heading: "Server management",
          paragraphs: [
            "GDLauncher includes built-in Minecraft server management, create a Vanilla, Forge, Fabric, NeoForge, or Quilt server and manage it from the same UI as your singleplayer instances. The CurseForge App doesn't include server management.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Both launchers can share a setup with a friend. The CurseForge App keeps everything inside the CurseForge ecosystem, you can hand off a CurseForge modpack but a setup that mixes CurseForge mods with Modrinth mods can't travel intact. GDLauncher's Cloud Instance Sharing accepts the mixed case: paste one code, the recipient gets your exact instance with files from both platforms re-downloaded from their original CDNs.",
          ],
        },
      ],
    },
    "modrinth-app": {
      title: "GDLauncher vs Modrinth App",
      description:
        "GDLauncher vs Modrinth App: which Minecraft launcher is best for mods and modpacks? Comparison of features, platforms, and ecosystem support.",
      intro:
        "The Modrinth App is the official Modrinth launcher and a great choice if you only use Modrinth content. GDLauncher integrates with Modrinth too, and adds CurseForge, Cloud Instance Sharing, and server management. Here's the side-by-side.",
      rows: [
        { feature: "CurseForge support", gdl: "Yes", competitor: "No" },
        {
          feature: "Modrinth support",
          gdl: "Yes",
          competitor: "Yes (native, it's their app)",
        },
        { feature: "Auto Java management", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto mod updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Auto modpack updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Multi-instance", gdl: "Yes", competitor: "Yes" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Yes (one-click code, mixed CF + MR)",
          competitor: "No (manual export, Modrinth only)",
        },
        { feature: "Server management", gdl: "Yes (built-in)", competitor: "Yes (Modrinth Hosting)" },
        { feature: "Modern UI", gdl: "Yes", competitor: "Yes" },
        { feature: "Source on GitHub", gdl: "Yes", competitor: "Yes" },
        { feature: "Pays addon authors", gdl: "Yes", competitor: "Yes" },
        { feature: "Lightweight", gdl: "Medium", competitor: "Medium" },
      ],
      verdict:
        "The Modrinth App is fantastic if you live entirely in the Modrinth ecosystem. But many of the most popular modpacks (RLCraft, ATM10, DawnCraft, the FTB lineup) are still CurseForge-only, and even cross-platform packs are usually CurseForge-first. GDLauncher gives you Modrinth plus CurseForge in one browser, plus Cloud Instance Sharing for friends, plus built-in server management. Pick GDLauncher if you want the broader ecosystem; pick Modrinth App if you want a focused, Modrinth-only experience.",
      sections: [
        {
          heading: "The CurseForge gap",
          paragraphs: [
            "The biggest difference is straightforward: the Modrinth App can't install CurseForge content. For mods that are Modrinth-only, this doesn't matter. But CurseForge still hosts the larger modpack library and many older Forge mods exclusively. GDLauncher's browser shows both platforms in one search, so you can pick whichever has the version you need.",
          ],
        },
        {
          heading: "Both ecosystems are great",
          paragraphs: [
            "Modrinth has a smaller library but a faster, ad-free site and better APIs for modders. CurseForge has the deeper catalog and historical packs. Most popular mods are now on both. GDLauncher's strategy is to support both natively rather than force you to choose.",
          ],
        },
        {
          heading: "Server management",
          paragraphs: [
            "Modrinth's server management is the paid Modrinth Hosting integration: you provision a server through Modrinth and manage it from the app. GDLauncher's server management is local, create a Vanilla / Forge / Fabric / NeoForge / Quilt server on your own machine, watch the live console, and edit the same instance settings you use for singleplayer, no hosting bill required.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "The other GDLauncher feature the Modrinth App doesn't replicate. Paste a code, get the exact setup with mixed CurseForge + Modrinth content in a single share.",
          ],
        },
      ],
    },
    atlauncher: {
      title: "GDLauncher vs ATLauncher",
      description:
        "GDLauncher vs ATLauncher: detailed comparison of UI, modpack support, server management, and developer experience. Which is the better Minecraft launcher?",
      intro:
        "ATLauncher is a long-running Java-based modpack launcher with its own ATLauncher pack ecosystem. GDLauncher is the newer Rust + Solid alternative with a modern UI and one-click CurseForge / Modrinth installs. Here's how they compare.",
      rows: [
        {
          feature: "CurseForge support",
          gdl: "Yes",
          competitor: "Partial (workaround)",
          note: "When a mod opts out of third-party API access, ATLauncher asks you to download that file manually in a browser",
        },
        { feature: "Modrinth support", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto Java management", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto mod updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Auto modpack updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Multi-instance", gdl: "Yes", competitor: "Yes" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Yes (one-click code, mixed CF + MR)",
          competitor: "No (manual export, no mixed CF + MR)",
        },
        { feature: "Server management", gdl: "Yes (built-in)", competitor: "No" },
        {
          feature: "Modern UI",
          gdl: "Yes",
          competitor: "Partial (Java Swing with FlatLaf)",
        },
        { feature: "Pays addon authors", gdl: "Yes", competitor: "No" },
        { feature: "Source on GitHub", gdl: "Yes", competitor: "Yes" },
        {
          feature: "Custom modpack publishing",
          gdl: "Yes (via Cloud Instance Sharing code)",
          competitor: "Yes (ATLauncher packs)",
        },
      ],
      verdict:
        "ATLauncher is a solid choice if you specifically want ATLauncher's curated pack list or you're already used to its workflow. GDLauncher's strengths are a more modern UI, deeper CurseForge integration, Cloud Instance Sharing, and built-in server management. For most modded Minecraft players in 2026, GDLauncher's experience is closer to what you'd expect from a modern app.",
      sections: [
        {
          heading: "UI generation gap",
          paragraphs: [
            "ATLauncher uses Java Swing with the modern FlatLaf look-and-feel layered on top. That's a real step up from classic Swing, but it still trails native modern launchers on density, motion, and platform feel. GDLauncher is built with Solid and uses a custom UnoCSS-based design system with native-feeling drag and drop, animations, and grouping.",
          ],
        },
        {
          heading: "CurseForge integration",
          paragraphs: [
            "ATLauncher and GDLauncher both browse and install CurseForge packs from inside the launcher, so the everyday experience is similar. The friction lives at the edges: when a mod author has opted out of third-party API access for their file, ATLauncher asks you to click through each blocked link and download those files manually in a browser. GDLauncher's CurseForge partnership fetches those files directly, so installs stay one-click even when packs include blocked mods.",
          ],
        },
        {
          heading: "ATLauncher packs vs Cloud Instance Sharing",
          paragraphs: [
            "ATLauncher hosts its own pack ecosystem. GDLauncher doesn't compete on that, instead, Cloud Instance Sharing lets anyone share their exact setup (mods, configs, settings) with a single code. Different philosophies; pick what fits how you and your friends play.",
          ],
        },
      ],
    },
    multimc: {
      title: "GDLauncher vs MultiMC",
      description:
        "GDLauncher vs MultiMC: detailed comparison of features, automation, modpack handling, and modern UI. Find the right Minecraft launcher for you.",
      intro:
        "MultiMC pioneered multi-instance Minecraft launching, though its last official release was 0.6.14 in December 2021 and most active development has moved to its forks (Prism Launcher chief among them). GDLauncher is a modern, opinionated launcher with deep automation. Here's the practical comparison.",
      rows: [
        {
          feature: "CurseForge support",
          gdl: "Yes",
          competitor: "No",
        },
        { feature: "Modrinth support", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto Java management", gdl: "Yes", competitor: "No" },
        { feature: "Auto mod updates", gdl: "Yes", competitor: "No" },
        { feature: "Auto modpack updates", gdl: "Yes", competitor: "No" },
        {
          feature: "Multi-instance",
          gdl: "Yes",
          competitor: "Yes (its specialty)",
        },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Yes (one-click code, mixed CF + MR)",
          competitor: "No (manual export, no mixed CF + MR)",
        },
        { feature: "Server management", gdl: "Yes (built-in)", competitor: "No" },
        { feature: "Modern UI", gdl: "Yes", competitor: "No" },
        { feature: "Pays addon authors", gdl: "Yes", competitor: "No" },
        { feature: "Source on GitHub", gdl: "Yes", competitor: "Yes" },
        { feature: "Lightweight", gdl: "No", competitor: "Yes (very)" },
      ],
      verdict:
        "MultiMC is a great choice if you want a tiny, hyper-flexible launcher and are happy doing your own Java setup, mod management, and updates. GDLauncher is for players who'd rather have those things handled automatically, auto Java, auto updates, one-click installs, Cloud Instance Sharing, and server management, without sacrificing the multi-instance workflow MultiMC pioneered.",
      sections: [
        {
          heading: "Automation vs control",
          paragraphs: [
            "MultiMC's design is \"do nothing the user didn't ask for.\" That means you set the Java path, you pick the version, you manage mods, you update them. Power users love this. New players bounce.",
            "GDLauncher takes the opposite approach: detect what each instance needs, install it, keep it updated, but expose all the same knobs in instance settings if you want to override anything. The defaults work; the controls are still there.",
          ],
        },
        {
          heading: "Modpack handling",
          paragraphs: [
            "MultiMC has a built-in Modrinth browser, but no CurseForge integration. To play CurseForge packs you'd need to import them manually as zip files, or use third-party tools to fetch the manifest. GDLauncher's browser shows CurseForge and Modrinth side by side, with one-click installs on both.",
          ],
        },
        {
          heading: "The legacy",
          paragraphs: [
            "MultiMC hasn't shipped a new release since December 2021; the project's energy has effectively moved into Prism Launcher and other forks. If you've used MultiMC for years and want a more modern UI without losing the workflow, Prism is the natural upgrade path; GDLauncher is the bigger jump (more automation, fewer manual steps). Try both and pick the model that fits how you actually use a launcher.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "Sharing a setup with a friend in MultiMC means exporting the instance to a zip and handing the file over. That works, but it's a file you have to host somewhere, and the recipient has to import it the same way. GDLauncher's Cloud Instance Sharing replaces that with a short code: paste it, the launcher pulls the snapshot from the GDL service, and mods re-download from their original CDNs. One code, mixed CurseForge + Modrinth content in the same share, no zip file to pass around.",
          ],
        },
      ],
    },
    "ftb-app": {
      title: "GDLauncher vs FTB App",
      description:
        "GDLauncher vs FTB App: how the launchers compare on Modrinth support, Cloud Instance Sharing, and built-in server management.",
      intro:
        "The FTB App is the launcher from the Feed The Beast team, focused on FTB's own curated pack list plus CurseForge browsing. GDLauncher is the modern Rust + Solid alternative with one-click installs from both CurseForge and Modrinth. Here's how they stack up.",
      rows: [
        { feature: "CurseForge support", gdl: "Yes", competitor: "Yes" },
        { feature: "Modrinth support", gdl: "Yes", competitor: "No" },
        { feature: "Auto Java management", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto mod updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Auto modpack updates", gdl: "Yes", competitor: "Yes (with prompt)" },
        { feature: "Multi-instance", gdl: "Yes", competitor: "Yes" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Yes (one-click code, mixed CF + MR)",
          competitor: "No (manual export, no mixed CF + MR)",
        },
        { feature: "Server management", gdl: "Yes (built-in)", competitor: "No" },
        { feature: "Modern UI", gdl: "Yes", competitor: "Yes" },
        { feature: "Pays addon authors", gdl: "Yes", competitor: "Yes" },
        { feature: "Source on GitHub", gdl: "Yes", competitor: "Yes" },
      ],
      verdict:
        "The FTB App is solid if you mostly install FTB's own packs and don't mind being CurseForge-only on the third-party side. GDLauncher gives you both CurseForge and Modrinth in the same browser, one-click Cloud Instance Sharing for mixed CF + MR setups, and built-in server management with live console. For most modded Minecraft players in 2026, GDLauncher's experience is closer to what you'd expect from a modern app.",
      sections: [
        {
          heading: "The FTB pack ecosystem",
          paragraphs: [
            "The FTB App's strength is the curated FTB modpack list, well-tested across versions and updated alongside the wider Feed The Beast community. The app can also browse and install third-party CurseForge packs. GDLauncher doesn't curate its own pack list, the built-in browser searches CurseForge and Modrinth side by side instead, so you pick the pack and the platform follows.",
          ],
        },
        {
          heading: "Modrinth and the broader catalog",
          paragraphs: [
            "The FTB App is CurseForge-only on the third-party side. Modrinth has been growing fast, especially for Fabric mods, performance mods, and shaders, with many authors now publishing to both platforms. GDLauncher's browser shows both at once, so you don't have to pick which ecosystem to live in.",
          ],
        },
        {
          heading: "Server management",
          paragraphs: [
            "The FTB App has a server installer that downloads a server pack as a folder you launch and manage yourself via command line, there's no in-app management. GDLauncher includes a full server view inside the launcher: create a Vanilla / Forge / Fabric / NeoForge / Quilt server, watch the live console, manage players, and edit the same instance settings you use for singleplayer.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "If you want to hand off a custom setup in the FTB App, you're back to exporting an instance and sending the file. GDLauncher's Cloud Instance Sharing turns that into a one-click code your friend pastes inside their launcher. The share works across CurseForge and Modrinth content in the same instance, so a setup that mixes FTB-style CurseForge mods with Modrinth-only additions still travels intact.",
          ],
        },
      ],
    },
    tlauncher: {
      title: "GDLauncher vs TLauncher",
      description:
        "GDLauncher vs TLauncher: authentication, official server access, modpack support, and what Mojang's EULA actually requires.",
      intro:
        "TLauncher is an unofficial Minecraft launcher that runs the game without going through Microsoft account authentication. Because it skips the official sign-in, it can't reach servers or services that verify identity (Hypixel, Realms, and most public servers), and you trade away the support and ecosystem access that come with a verified account. GDLauncher uses the official Microsoft sign-in flow. Here's how the two compare, and what you give up.",
      rows: [
        {
          feature: "Official Microsoft sign-in",
          gdl: "Yes",
          competitor: "No (auth bypass)",
        },
        {
          feature: "Access to official servers (Hypixel, Mineplex, etc.)",
          gdl: "Yes",
          competitor: "No",
        },
        { feature: "Access to Minecraft Realms", gdl: "Yes", competitor: "No" },
        { feature: "CurseForge support", gdl: "Yes", competitor: "No" },
        { feature: "Modrinth support", gdl: "Yes", competitor: "No" },
        { feature: "Auto Java management", gdl: "Yes", competitor: "Yes" },
        { feature: "Auto mod updates", gdl: "Yes", competitor: "No" },
        { feature: "Auto modpack updates", gdl: "Yes", competitor: "No" },
        {
          feature: "Cloud Instance Sharing",
          gdl: "Yes (one-click code, mixed CF + MR)",
          competitor: "No",
        },
        { feature: "Server management", gdl: "Yes (built-in)", competitor: "No" },
        { feature: "Pays addon authors", gdl: "Yes", competitor: "No" },
        { feature: "Source on GitHub", gdl: "Yes", competitor: "No" },
      ],
      verdict:
        "TLauncher's headline appeal is that it runs Minecraft without authentication. The catch is that anything requiring a real Microsoft identity (Hypixel-class public servers, Realms, mod platforms that fund authors) is off the table, you give up the support and security that come with a verified account, and you take on whatever your local rules say about using unlicensed software. GDLauncher takes the official path: sign in with Microsoft once, and you get the full ecosystem, with the modpack and Cloud Instance Sharing workflow layered on top.",
      sections: [
        {
          heading: "Authentication and what it unlocks",
          paragraphs: [
            "TLauncher launches Minecraft without signing into a Mojang or Microsoft account. That makes the game run for users who don't have a license, but it also means servers that verify identity (Hypixel, Mineplex, and almost every popular public server) won't let you in. Minecraft Realms requires authentication too, so those don't work either.",
            "GDLauncher uses the official Microsoft sign-in flow. You sign in once, and you have the same identity everywhere: official servers, Realms, modded servers, your friends' worlds. The launcher is the entry point, authentication is what unlocks the actual ecosystem.",
          ],
        },
        {
          heading: "Official vs unofficial",
          paragraphs: [
            "Minecraft is sold by Microsoft and the official launchers (including partners like GDLauncher) sign you in with a paid account. TLauncher skips that step. We can't speak to what that means under your local rules, but the path that's supported, ecosystem-compatible, and free of update-day surprises is the official one.",
            "If you can afford a Minecraft license, that's the simpler choice: it unlocks the whole ecosystem and you're never one update or server change away from things breaking. If you can't, Mojang and Microsoft offer legitimate free options (older demo versions, Minecraft Education trials in some regions) worth looking into first.",
          ],
        },
        {
          heading: "Mod and modpack ecosystem",
          paragraphs: [
            "TLauncher ships with a bundled mod list, but it doesn't integrate with CurseForge or Modrinth, the two platforms where most modders publish. That means you don't get the modpack catalog and the platforms' revenue share never reaches the mod authors. GDLauncher integrates with both directly: search, install, and update from inside the launcher.",
          ],
        },
        {
          heading: "Cloud Instance Sharing",
          paragraphs: [
            "TLauncher has no first-party way to share a configured setup with a friend, you're stuck zipping a folder and hoping the recipient can reassemble it. GDLauncher's Cloud Instance Sharing turns a configured instance into a short code: your friend pastes it inside their launcher, the snapshot pulls from the GDL service, and mods re-download from CurseForge and Modrinth. It's the kind of friction-free hand-off that's hard to retrofit onto an auth-skipping launcher in the first place.",
          ],
        },
      ],
    },
  },
}

// ---------------------------------------------------------------------------
// Per-locale data dictionaries.
// All non-English locales fall back to English if a slug is missing.
// ---------------------------------------------------------------------------

import jaData from "./vsData.ja"
import koData from "./vsData.ko"
import deData from "./vsData.de"
import frData from "./vsData.fr"
import esData from "./vsData.es"
import ptBRData from "./vsData.pt-BR"
import itData from "./vsData.it"

const data: Record<Locale, LocaleData> = {
  en,
  ja: jaData,
  ko: koData,
  de: deData,
  fr: frData,
  es: esData,
  "pt-BR": ptBRData,
  it: itData,
}

export function getComparisonChrome(locale: Locale): ComparisonChrome {
  return (data[locale] ?? data[DEFAULT_LOCALE]).chrome
}

export function getComparisonHub(locale: Locale): ComparisonHubData {
  return (data[locale] ?? data[DEFAULT_LOCALE]).hub
}

export function getComparison(
  locale: Locale,
  slug: ComparisonSlug,
): ComparisonData {
  const localeData = data[locale] ?? data[DEFAULT_LOCALE]
  return localeData.comparisons[slug] ?? (data[DEFAULT_LOCALE].comparisons[slug] as ComparisonData)
}
