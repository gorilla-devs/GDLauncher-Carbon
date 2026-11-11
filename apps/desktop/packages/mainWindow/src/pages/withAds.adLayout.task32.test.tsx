import { expect, it, vi } from "vitest"
import { createContext } from "solid-js"
import { render } from "solid-js/web"

vi.mock("@/components/AdBanner", () => ({ AdsBanner: () => <div /> }))
vi.mock("@/components/TopBannerAd", () => ({ TopBannerAd: () => <div /> }))
vi.mock("@/components/Navbar", () => ({ default: () => <nav /> }))
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
vi.mock("@gd/i18n", () => ({ Trans: (props: any) => <span>{props.key}</span> }))
vi.mock("@gd/ui", () => ({
  Button: (props: any) => <button>{props.children}</button>
}))

// adhelper asks main for the ad size as soon as it is loaded.
Object.assign(window, {
  getAdSize: () =>
    Promise.resolve({ adSize: { width: 400, height: 600, shouldShow: true } }),
  adSizeChanged: () => {}
})

const { default: withAdsLayout } = await import("./withAds")
const { adLayoutMounted } = await import("@/utils/adhelper")

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
