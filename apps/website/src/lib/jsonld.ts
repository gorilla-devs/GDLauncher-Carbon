/**
 * Structured-data (JSON-LD) builders for the GDLauncher site.
 *
 * Each function returns plain objects ready to be JSON.stringify'd into a
 * <script type="application/ld+json"> tag. PageShell / BaseHead accept arrays
 * of these via their `jsonLd` prop.
 *
 * Why centralized: structured data is repeated across pages and easy to break
 * subtly. Building objects from one place keeps schemas consistent and makes
 * it trivial to extend (add ratings, review counts, version numbers, etc.).
 */

import { localizedPath, type Locale } from "./i18n";

const SITE_URL = "https://gdlauncher.com";
const CDN_URL = "https://cdn.gdl.gg";
const DEFAULT_IMAGE = `${CDN_URL}/assets/website-preview.jpg`;

interface Socials {
  discord: string;
  github: string;
  instagram: string;
  twitter: string;
}

/**
 * Homepage: SoftwareApplication (the launcher itself), Organization (the
 * project / publisher), and WebSite (with a SearchAction so brand SERPs may
 * render a sitelinks search box).
 */
export function homepageJsonLd(opts: {
  locale: Locale;
  socials: Socials;
}): Record<string, unknown>[] {
  const { locale, socials } = opts;
  const homeUrl = new URL(localizedPath("/", locale), SITE_URL).toString();

  const softwareApp: Record<string, unknown> = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: "GDLauncher",
    alternateName: ["GDLauncher Carbon", "GD Launcher"],
    applicationCategory: "GameApplication",
    applicationSubCategory: "Minecraft Launcher",
    operatingSystem: "Windows, macOS, Linux",
    description:
      "Free Minecraft launcher for mods and modpacks with one-click installs from CurseForge and Modrinth. Auto-manages Java, dependencies, and updates.",
    url: homeUrl,
    image: DEFAULT_IMAGE,
    softwareVersion: "2",
    inLanguage: locale,
    offers: {
      "@type": "Offer",
      price: "0",
      priceCurrency: "USD",
    },
    featureList: [
      "One-click modpack installs",
      "CurseForge and Modrinth support",
      "Forge, Fabric, NeoForge, and Quilt mod loaders",
      "Automatic Java management",
      "Cloud Instance Sharing",
      "Built-in server management",
      "Automatic mod updates",
    ],
    screenshot: `${CDN_URL}/assets/website-home-app-screenshot.jpg`,
  };

  const organization: Record<string, unknown> = {
    "@context": "https://schema.org",
    "@type": "Organization",
    name: "GDLauncher",
    legalName: "GorillaDevs Inc.",
    url: SITE_URL,
    logo: DEFAULT_IMAGE,
    sameAs: [
      socials.discord,
      socials.github,
      socials.twitter,
      socials.instagram,
    ],
  };

  const website: Record<string, unknown> = {
    "@context": "https://schema.org",
    "@type": "WebSite",
    name: "GDLauncher",
    url: SITE_URL,
    potentialAction: {
      "@type": "SearchAction",
      target: {
        "@type": "EntryPoint",
        urlTemplate: `${SITE_URL}/?q={search_term_string}`,
      },
      "query-input": "required name=search_term_string",
    },
  };

  return [softwareApp, organization, website];
}

/**
 * BreadcrumbList, pass an ordered list of {name, url} crumbs (root first,
 * current page last). The Breadcrumb component already renders the visible
 * crumbs; this just emits the structured-data twin.
 */
export function breadcrumbsJsonLd(
  items: Array<{ name: string; url: string }>
): Record<string, unknown> {
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: items.map((item, idx) => ({
      "@type": "ListItem",
      position: idx + 1,
      name: item.name,
      item: item.url,
    })),
  };
}

/** HowTo, for guide pages with sequential install steps. */
export function howToJsonLd(opts: {
  name: string;
  description: string;
  url: string;
  steps: Array<{ name: string; text: string; url?: string }>;
  image?: string;
  totalTimeIso?: string; // ISO 8601 duration, e.g., "PT5M"
}): Record<string, unknown> {
  const { name, description, url, steps, image, totalTimeIso } = opts;
  return {
    "@context": "https://schema.org",
    "@type": "HowTo",
    name,
    description,
    image: image || DEFAULT_IMAGE,
    totalTime: totalTimeIso,
    estimatedCost: { "@type": "MonetaryAmount", currency: "USD", value: "0" },
    step: steps.map((s, idx) => ({
      "@type": "HowToStep",
      position: idx + 1,
      name: s.name,
      text: s.text,
      url: s.url ?? `${url}#step-${idx + 1}`,
    })),
  };
}

/**
 * Article, for blog posts. Emits Article schema with headline, dates,
 * publisher, and main entity. This is what unlocks the article-style
 * rich result in Google SERPs (large card with thumbnail and timestamp).
 */
export function articleJsonLd(opts: {
  title: string;
  description: string;
  url: string;
  image?: string;
  publishedTime?: string; // ISO 8601
  modifiedTime?: string; // ISO 8601
  author?: string;
}): Record<string, unknown> {
  return {
    "@context": "https://schema.org",
    "@type": "Article",
    headline: opts.title,
    description: opts.description,
    image: opts.image || DEFAULT_IMAGE,
    datePublished: opts.publishedTime,
    dateModified: opts.modifiedTime || opts.publishedTime,
    author: opts.author
      ? { "@type": "Person", name: opts.author }
      : { "@type": "Organization", name: "GDLauncher" },
    publisher: {
      "@type": "Organization",
      name: "GDLauncher",
      logo: { "@type": "ImageObject", url: DEFAULT_IMAGE },
    },
    mainEntityOfPage: { "@type": "WebPage", "@id": opts.url },
  };
}

/** FAQPage, pairs of {question, answer} rendered as rich SERP results. */
export function faqJsonLd(
  items: Array<{ question: string; answer: string }>
): Record<string, unknown> {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: items.map((item) => ({
      "@type": "Question",
      name: item.question,
      acceptedAnswer: { "@type": "Answer", text: item.answer },
    })),
  };
}

/**
 * Per-addon SoftwareApplication. Each addon page emits one of these so the
 * pages stop looking identical to Google's quality classifier, the schema
 * carries unique facts (name, author, MC versions, downloads, license, etc.).
 */
export function addonJsonLd(opts: {
  name: string;
  description: string;
  url: string;
  image?: string;
  author?: string | null;
  publisher: "CurseForge" | "Modrinth";
  category: string;
  downloads?: number | null;
  dateModified?: string | null;
  gameVersions?: string[] | null;
  license?: string | null;
}): Record<string, unknown> {
  const data: Record<string, unknown> = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: opts.name,
    applicationCategory: "GameApplication",
    applicationSubCategory: opts.category,
    description: opts.description,
    url: opts.url,
    operatingSystem: "Windows, macOS, Linux",
    isAccessibleForFree: true,
    offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
    publisher: { "@type": "Organization", name: opts.publisher },
  };
  if (opts.image) data.image = opts.image;
  if (opts.author) data.author = { "@type": "Person", name: opts.author };
  if (opts.downloads != null) data.downloadCount = opts.downloads;
  if (opts.dateModified) data.dateModified = opts.dateModified;
  if (opts.gameVersions?.length)
    data.gameVersion = opts.gameVersions.slice(0, 10).join(", ");
  if (opts.license) data.license = opts.license;
  return data;
}
