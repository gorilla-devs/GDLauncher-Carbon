import type {
  AnimationController,
  AnimationRefs,
  AnimationRefName,
  AnimationOptions,
  TransitionDirection
} from "../flow/types"
import type { Occasion } from "@/utils/occasions"
import { AnimationPresets } from "./AnimationTimeline"

/**
 * AnimationController Implementation
 *
 * Provides declarative animation primitives for the auth flow
 * All methods return promises for easy sequencing and composition
 */
export class AnimationControllerImpl implements AnimationController {
  private refs: AnimationRefs = {}
  private ready = false
  private readyCallbacks: (() => void)[] = []

  // Animation state tracking to prevent duplicate animations
  private backButtonState: "hidden" | "showing" | "shown" | "hiding" = "hidden"
  private skipButtonState: "hidden" | "showing" | "shown" | "hiding" = "hidden"

  // Track running animations for cancellation
  private backButtonAnimation: Animation | null = null
  private skipButtonAnimation: Animation | null = null

  private footerIntent = {
    backButton: { generation: 0, visible: undefined as boolean | undefined },
    skipButton: { generation: 0, visible: undefined as boolean | undefined }
  }

  constructor(private reducedMotion: boolean) {}

  // ============================================================================
  // Ref Management
  // ============================================================================

  registerRefs(newRefs: AnimationRefs): void {
    this.refs = { ...this.refs, ...newRefs }

    // Check if all essential refs are now available
    if (this.refsReady() && !this.ready) {
      this.ready = true
      // Notify all waiting callbacks
      this.readyCallbacks.forEach((cb) => cb())
      this.readyCallbacks = []
    }
  }

  refsReady(): boolean {
    return !!(
      this.refs.sidebar &&
      this.refs.video &&
      this.refs.backgroundBlur &&
      this.refs.loadingSpinner
    )
  }

  // ============================================================================
  // Step Animations
  // ============================================================================
  // NOTE: Step transitions are now handled by View Transition API in FlowController

  step = {
    /**
     * Slide transition between steps (stub - handled by View Transition API)
     */
    slideTransition: async (_direction: TransitionDirection): Promise<void> => {
      // Step transitions are handled by the View Transition API in FlowController
      // This is a stub implementation to satisfy the interface
      return Promise.resolve()
    }
  }

  // ============================================================================
  // Sidebar Animations
  // ============================================================================

  sidebar = {
    /**
     * Slide sidebar in from left, video to right
     * Also fades out loading spinner and background blur
     */
    slideIn: async (): Promise<void> => {
      await this.waitForRefs()

      const { sidebar, video, loadingSpinner, backgroundBlur } = this.refs

      if (this.reducedMotion) {
        sidebar!.style.transform = "translateX(0%)"
        video!.style.transform = "translateX(15%)"
        loadingSpinner!.style.opacity = "0"
        backgroundBlur!.style.opacity = "0"
        return
      }

      // Run all animations in parallel
      await Promise.all([
        AnimationPresets.slideX(sidebar!, "-100%", "0%", {
          duration: 300,
          delay: 200
        }),

        AnimationPresets.slideX(video!, "0%", "15%", {
          duration: 300,
          delay: 200
        }),

        AnimationPresets.fadeOut(loadingSpinner!, { duration: 300 }),

        AnimationPresets.fadeOut(backgroundBlur!, { duration: 500 })
      ])
    },

    /**
     * Slide sidebar out to left, video to center
     */
    slideOut: async (): Promise<void> => {
      await this.waitForRefs()

      const { sidebar, video } = this.refs

      if (this.reducedMotion) {
        sidebar!.style.transform = "translateX(-100%)"
        video!.style.transform = "translateX(0%)"
        return
      }

      await Promise.all([
        AnimationPresets.slideX(sidebar!, "0%", "-100%", { duration: 500 }),

        AnimationPresets.slideX(video!, "15%", "0%", { duration: 500 })
      ])
    }
  }

  // ============================================================================
  // Loading Overlay
  // ============================================================================

  loading = {
    /**
     * Show loading overlay (spinner + blur)
     */
    show: async (): Promise<void> => {
      await this.waitForRefs()

      const { loadingSpinner, backgroundBlur } = this.refs

      if (this.reducedMotion) {
        loadingSpinner!.style.opacity = "1"
        backgroundBlur!.style.opacity = "1"
        return
      }

      await Promise.all([
        AnimationPresets.fadeIn(loadingSpinner!, { duration: 300 }),
        AnimationPresets.fadeIn(backgroundBlur!, { duration: 300 })
      ])
    },

    /**
     * Hide loading overlay
     */
    hide: async (): Promise<void> => {
      await this.waitForRefs()

      const { loadingSpinner, backgroundBlur } = this.refs

      if (this.reducedMotion) {
        loadingSpinner!.style.opacity = "0"
        backgroundBlur!.style.opacity = "0"
        return
      }

      await Promise.all([
        AnimationPresets.fadeOut(loadingSpinner!, { duration: 300 }),
        AnimationPresets.fadeOut(backgroundBlur!, { duration: 300 })
      ])
    }
  }

  // ============================================================================
  // Text Animations
  // ============================================================================

  text = {
    /**
     * Fade in text element
     */
    fadeIn: async (
      ref: AnimationRefName,
      options: AnimationOptions = {}
    ): Promise<void> => {
      await this.waitForRefs()

      const element = this.refs[ref]
      if (!element) {
        console.warn(`[AnimationController] Text ref '${ref}' not available`)
        return
      }

      if (this.reducedMotion) {
        element.style.opacity = "1"
        return
      }

      await AnimationPresets.fadeIn(element, {
        duration: options.duration ?? 500,
        delay: options.delay ?? 0
      })
    },

    /**
     * Fade out text element(s)
     */
    fadeOut: async (
      ref: AnimationRefName | AnimationRefName[],
      options: AnimationOptions = {}
    ): Promise<void> => {
      await this.waitForRefs()

      const refs = Array.isArray(ref) ? ref : [ref]

      if (this.reducedMotion) {
        refs.forEach((r) => {
          const element = this.refs[r]
          if (element) {
            element.style.opacity = "0"
          }
        })
        return
      }

      await Promise.all(
        refs.map((r) => {
          const element = this.refs[r]
          if (!element) {
            console.warn(`[AnimationController] Text ref '${r}' not available`)
            return Promise.resolve()
          }

          return AnimationPresets.fadeOut(element, {
            duration: options.duration ?? 500,
            delay: options.delay ?? 0
          })
        })
      )
    }
  }

  // ============================================================================
  // Background Blur
  // ============================================================================

  backgroundBlur = {
    /**
     * Fade in background blur
     */
    fadeIn: async (options: AnimationOptions = {}): Promise<void> => {
      await this.waitForRefs()

      const element = this.refs.backgroundBlur
      if (!element) return

      if (this.reducedMotion) {
        element.style.opacity = "1"
        return
      }

      await AnimationPresets.fadeIn(element, {
        duration: options.duration ?? 500,
        delay: options.delay ?? 0
      })
    },

    /**
     * Fade out background blur
     */
    fadeOut: async (options: AnimationOptions = {}): Promise<void> => {
      await this.waitForRefs()

      const element = this.refs.backgroundBlur
      if (!element) return

      if (this.reducedMotion) {
        element.style.opacity = "0"
        return
      }

      await AnimationPresets.fadeOut(element, {
        duration: options.duration ?? 500,
        delay: options.delay ?? 0
      })
    }
  }

  // ============================================================================
  // Back Button
  // ============================================================================

  backButton = {
    show: (): Promise<void> => this.animateFooter("backButton", true),
    hide: (): Promise<void> => this.animateFooter("backButton", false)
  }

  // ============================================================================
  // Skip Button
  // ============================================================================

  skipButton = {
    show: (): Promise<void> => this.animateFooter("skipButton", true),
    hide: (): Promise<void> => this.animateFooter("skipButton", false)
  }

  private async animateFooter(
    name: "backButton" | "skipButton",
    visible: boolean
  ): Promise<void> {
    const intent = this.footerIntent[name]
    const stateKey =
      name === "backButton" ? "backButtonState" : "skipButtonState"
    const animationKey =
      name === "backButton" ? "backButtonAnimation" : "skipButtonAnimation"
    const finalState = visible ? "shown" : "hidden"
    if (
      intent.visible === visible ||
      (intent.visible === undefined && this[stateKey] === finalState)
    )
      return

    // Accept opposing intent before waiting: rendered state alone cannot
    // establish ownership while an older request is still waiting for refs.
    const generation = ++intent.generation
    intent.visible = visible
    const owns = () =>
      intent.generation === generation && intent.visible === visible
    const predecessor = this[animationKey]
    if (predecessor) {
      predecessor.cancel()
      if (owns() && this[animationKey] === predecessor)
        this[animationKey] = null
    }

    const width = name === "backButton" ? "60%" : "40%"
    const margin = name === "backButton" ? "0 1rem 0 0" : "0"
    if (this.reducedMotion) {
      const element = this.refs[name]
      if (!element) {
        if (owns()) intent.visible = undefined
        return
      }
      element.style.width = visible ? width : "0"
      element.style.margin = visible ? margin : "0"
      this[stateKey] = finalState
      return
    }

    // Revoking a show that never started needs no hide animation.
    if (this[stateKey] === finalState && !predecessor) return
    await this.waitForRefs()
    if (!owns()) return
    const element = this.refs[name]
    if (!element) {
      intent.visible = undefined
      if (visible) console.warn(`[AnimationController] ${name} ref not found`)
      return
    }

    this[stateKey] = visible ? "showing" : "hiding"
    const animation = element.animate(
      visible
        ? [
            { width: "0", margin: "0" },
            { width, margin }
          ]
        : [
            { width, margin },
            { width: "0", margin: "0" }
          ],
      {
        duration: 300,
        easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
        fill: "forwards"
      }
    )
    this[animationKey] = animation
    const ownsAnimation = () =>
      intent.generation === generation && this[animationKey] === animation
    try {
      await animation.finished
      if (ownsAnimation()) this[stateKey] = finalState
    } catch {
      // Cancellation is expected. Only a current failure releases its intent;
      // an older AbortError must leave replacement state and handle untouched.
      if (ownsAnimation()) intent.visible = undefined
    } finally {
      if (ownsAnimation()) this[animationKey] = null
    }
  }

  // ============================================================================
  // Seasonal Splash
  // ============================================================================

  seasonal = {
    /**
     * Show seasonal splash message
     */
    show: async (_occasion: Occasion): Promise<void> => {
      if (this.reducedMotion) return

      // Seasonal splash animations would go here
      // This is placeholder for the actual implementation
    },

    /**
     * Hide seasonal splash
     */
    hide: async (): Promise<void> => {
      if (this.reducedMotion) return

      // Seasonal splash hide animations
    }
  }

  // ============================================================================
  // Private Helpers
  // ============================================================================

  /**
   * Wait for essential refs to be registered
   * Returns immediately if refs are already available
   */
  private waitForRefs(): Promise<void> {
    if (this.ready) return Promise.resolve()

    return new Promise((resolve) => {
      this.readyCallbacks.push(resolve)
    })
  }
}

/**
 * Create an animation controller instance
 */
export function createAnimationController(
  reducedMotion: boolean
): AnimationController {
  return new AnimationControllerImpl(reducedMotion)
}
