/**
 * Hook for managing welcome screen animations
 *
 * Coordinates complex multi-step animations when transitioning from login to library
 */
export function useAuthAnimations(props: {
  isFirstLaunch: boolean
  onAnimationComplete: () => void
}) {
  let sidebarRef: HTMLDivElement | undefined
  let backgroundBlurRef: HTMLDivElement | undefined
  let welcomeToTextRef: HTMLDivElement | undefined
  let gdlauncherTextRef: HTMLDivElement | undefined
  let videoRef: HTMLVideoElement | undefined

  /**
   * Orchestrate the welcome animation sequence
   *
   * Animation timeline:
   * 0ms:    Sidebar slides out left (-100%)
   * 0ms:    Video slides right (15% → 0%)
   * 350ms:  Background blur fades in
   * 1100ms: "Welcome to" text fades in
   * 2300ms: "GDLauncher" text fades in
   * 5000ms: Navigate to library
   */
  async function playWelcomeAnimation() {
    return new Promise<void>((resolve) => {
      if (!backgroundBlurRef || !props.isFirstLaunch) {
        // Skip animation if not first launch
        props.onAnimationComplete()
        resolve()
        return
      }

      // 1. Sidebar slides out to the left
      sidebarRef?.animate(
        [{ transform: "translateX(0%)" }, { transform: "translateX(-100%)" }],
        {
          duration: 500,
          easing: "linear",
          fill: "forwards"
        }
      )

      // 2. Video slides to center
      videoRef?.animate(
        [{ transform: "translateX(15%)" }, { transform: "translateX(0%)" }],
        {
          duration: 300,
          easing: "linear",
          fill: "forwards"
        }
      )

      // 3. Background blur fades in
      backgroundBlurRef.animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: 500,
        delay: 350,
        easing: "linear",
        fill: "forwards"
      })

      // 4. "Welcome to" text fades in
      welcomeToTextRef?.animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: 600,
        delay: 1100,
        easing: "linear",
        fill: "forwards"
      })

      // 5. "GDLauncher" text fades in
      gdlauncherTextRef?.animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: 600,
        delay: 2300,
        easing: "linear",
        fill: "forwards"
      })

      // 6. Complete and navigate after 5 seconds
      setTimeout(() => {
        props.onAnimationComplete()
        resolve()
      }, 5000)
    })
  }

  /**
   * Set sidebar element ref
   */
  function setSidebarRef(el: HTMLDivElement) {
    sidebarRef = el
  }

  /**
   * Set background blur element ref
   */
  function setBackgroundBlurRef(el: HTMLDivElement) {
    backgroundBlurRef = el
  }

  /**
   * Set "Welcome to" text element ref
   */
  function setWelcomeToTextRef(el: HTMLDivElement) {
    welcomeToTextRef = el
  }

  /**
   * Set "GDLauncher" text element ref
   */
  function setGdlauncherTextRef(el: HTMLDivElement) {
    gdlauncherTextRef = el
  }

  /**
   * Set video element ref
   */
  function setVideoRef(el: HTMLVideoElement) {
    videoRef = el
  }

  return {
    // Animation method
    playWelcomeAnimation,

    // Ref setters
    setSidebarRef,
    setBackgroundBlurRef,
    setWelcomeToTextRef,
    setGdlauncherTextRef,
    setVideoRef
  }
}
