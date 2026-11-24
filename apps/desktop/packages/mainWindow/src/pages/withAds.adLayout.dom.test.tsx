import { expect, it, vi } from "vitest"
import { createContext } from "solid-js"
import { render } from "solid-js/web"

vi.mock("@/components/AdBanner", () => ({ AdsBanner: () => <div /> }))
vi.mock("@/components/TopBannerAd", () => ({ TopBannerAd: () => <div /> }))
vi.mock("@/components/Navbar", () => ({ default: () => <nav /> }))
vi.mock("@/components/NavRail", () => ({ default: () => <nav /> }))
vi.mock("@/components/EnhancedSearchBar", () => ({
  EnhancedSearchBar: () => <div />
}))
vi.mock("@solidjs/router", () => ({
  useLocation: () => ({ pathname: "/library" })
}))
vi.mock("@/managers/NavigationManager", () => ({ PAGE_ID: "page" }))
vi.mock("@/themes/DecorationSlot", () => ({ DecorationSlot: () => null }))
vi.mock("@/themes/effects/EffectsLayer", () => ({ EffectsLayer: () => null }))
vi.mock("@/components/ThemedPatternSVG", () => ({ default: () => <svg /> }))
vi.mock("@/managers/ModalsManager", () => ({ useModal: () => undefined }))
vi.mock("@/components/SearchInputContext", () => ({
  SearchInputContext: createContext()
}))
vi.mock("@/utils/platformSearch", () => ({ getSearchResults: () => ({}) }))
vi.mock("@/utils/rspcClient", () => ({
  rspc: { createQuery: () => ({ data: undefined }) }
}))
vi.mock("@/utils/formatBytes", () => ({ formatBytes: () => "" }))
vi.mock("@gd/i18n", () => ({
  Trans: (props: any) => <span>{props.key}</span>,
  useTransContext: () => [(key: string) => key]
}))
vi.mock("@gd/ui", () => ({
  Button: (props: any) => <button>{props.children}</button>
}))

// adhelper asks main for the ad size as soon as it is loaded.
Object.assign(window, {
  getAdSize: () =>
    Promise.resolve({ adSize: { width: 400, height: 600, shouldShow: true } }),
  adSizeChanged: () => {},
  // The ad itself is main's own view; the page only labels it.
  adView: { setLabels: () => {}, onOpenWhyAds: () => () => {} }
})

const { default: withAdsLayout } = await import("./withAds")
const { adLayoutMounted, _setAdSize } = await import("@/utils/adhelper")

it("tells the shutdown prompt the ad column is on screen while the ads layout is mounted", () => {
  expect(adLayoutMounted()).toBe(false)
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(
    () => withAdsLayout({ children: <div>page</div> }),
    root
  )
  try {
    expect(adLayoutMounted()).toBe(true)
  } finally {
    dispose()
    root.remove()
  }
  expect(adLayoutMounted()).toBe(false)
})

it("puts the ad column beside the navbar on a short display without rebuilding the page", async () => {
  await Promise.resolve()
  _setAdSize({ width: 160, height: 600, shouldShow: true, besideNavbar: false })
  const root = document.createElement("div")
  document.body.append(root)
  const dispose = render(
    () => withAdsLayout({ children: <div data-testid="page">page</div> }),
    root
  )
  const find = (testId: string) =>
    root.querySelector<HTMLElement>(`[data-testid="${testId}"]`)!
  // The part of the row the page lives in, left of the ad column.
  const contentColumn = () =>
    find("page").closest('[data-gdl="content"]')!.parentElement
  try {
    const page = find("page")
    const column = find("ads-column")
    // Under the navbar: in the row, as tall as the row, navbar full width.
    expect(column.classList.contains("fixed")).toBe(false)
    expect(column.classList.contains("h-full")).toBe(true)
    expect(find("ads-layout-top").style.width).toBe("")
    expect(column.previousElementSibling).toBe(contentColumn())

    _setAdSize({
      width: 160,
      height: 600,
      shouldShow: true,
      besideNavbar: true
    })
    // Beside the navbar: the whole height of the window at its right edge,
    // with the navbar ending where the column starts.
    expect(column.classList.contains("fixed")).toBe(true)
    expect(column.classList.contains("h-screen")).toBe(true)
    expect(column.classList.contains("h-full")).toBe(false)
    expect(find("ads-layout-top").style.width).toBe("calc(100vw - 160px)")
    // Its place in the row is kept, so the page keeps its width.
    const spacer = column.previousElementSibling as HTMLElement
    expect(spacer).not.toBe(contentColumn())
    expect(spacer.style.width).toBe("160px")
    // Moving to another display must not reload the page or the ad.
    expect(find("page")).toBe(page)
    expect(find("ads-column")).toBe(column)

    _setAdSize({
      width: 400,
      height: 600,
      shouldShow: true,
      besideNavbar: false
    })
    expect(column.classList.contains("fixed")).toBe(false)
    expect(find("ads-layout-top").style.width).toBe("")
    expect(column.previousElementSibling).toBe(contentColumn())
    expect(find("page")).toBe(page)
  } finally {
    dispose()
    root.remove()
  }
})
