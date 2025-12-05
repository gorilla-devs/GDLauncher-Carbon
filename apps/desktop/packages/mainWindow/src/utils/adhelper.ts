import { createSignal } from "solid-js"
import { createStore } from "solid-js/store"

export interface BoundsSize {
  width: number
  height: number
  shouldShow: boolean
}

export const [adSize, _setAdSize] = createStore<BoundsSize>({
  width: 0,
  height: 0,
  shouldShow: true
})

export const [bannerAdSize, _setBannerAdSize] = createStore<BoundsSize>({
  width: 0,
  height: 0,
  shouldShow: false
})

// Signal for hiding the "Why are ads needed" text on constrained displays
export const [hideAdText, _setHideAdText] = createSignal(false)

const init = async () => {
  const bounds = await window.getAdSize()
  _setAdSize(bounds.adSize)

  if (bounds.bannerAdSize) {
    _setBannerAdSize(bounds.bannerAdSize)
  }

  if (bounds.hideAdText !== undefined) {
    _setHideAdText(bounds.hideAdText)
  }

  window.adSizeChanged(
    (
      _,
      newBounds: {
        adSize: Omit<BoundsSize, "shouldShow">
        bannerAdSize?: Omit<BoundsSize, "shouldShow">
        hideAdText?: boolean
      }
    ) => {
      _setAdSize({
        ...newBounds.adSize,
        shouldShow: false
      })

      setTimeout(() => {
        _setAdSize({
          ...newBounds.adSize,
          shouldShow: true
        })
      }, 100)

      if (newBounds.bannerAdSize) {
        _setBannerAdSize({
          ...newBounds.bannerAdSize,
          shouldShow: false
        })

        setTimeout(() => {
          _setBannerAdSize({
            ...newBounds.bannerAdSize,
            shouldShow: true
          })
        }, 100)
      }

      if (newBounds.hideAdText !== undefined) {
        _setHideAdText(newBounds.hideAdText)
      }
    }
  )
}

init()

export default adSize
