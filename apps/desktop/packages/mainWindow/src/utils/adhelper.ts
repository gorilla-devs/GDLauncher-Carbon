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

const init = async () => {
  const bounds = await window.getAdSize()
  _setAdSize(bounds.adSize)

  if (bounds.bannerAdSize) {
    _setBannerAdSize(bounds.bannerAdSize)
  }

  window.adSizeChanged(
    (
      _,
      newBounds: {
        adSize: Omit<BoundsSize, "shouldShow">
        bannerAdSize?: Omit<BoundsSize, "shouldShow">
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
    }
  )
}

init()

export default adSize
