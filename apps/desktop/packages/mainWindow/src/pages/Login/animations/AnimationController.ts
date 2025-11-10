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
      if (this.reducedMotion) return

      await this.waitForRefs()

      const { sidebar, video, loadingSpinner, backgroundBlur } = this.refs

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
      if (this.reducedMotion) return

      await this.waitForRefs()

      const { sidebar, video } = this.refs

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
      if (this.reducedMotion) return

      await this.waitForRefs()

      const { loadingSpinner, backgroundBlur } = this.refs

      await Promise.all([
        AnimationPresets.fadeIn(loadingSpinner!, { duration: 300 }),
        AnimationPresets.fadeIn(backgroundBlur!, { duration: 300 })
      ])
    },

    /**
     * Hide loading overlay
     */
    hide: async (): Promise<void> => {
      if (this.reducedMotion) return

      await this.waitForRefs()

      const { loadingSpinner, backgroundBlur } = this.refs

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
      if (this.reducedMotion) return

      await this.waitForRefs()

      const element = this.refs[ref]
      if (!element) {
        console.warn(`[AnimationController] Text ref '${ref}' not available`)
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
      if (this.reducedMotion) return

      await this.waitForRefs()

      const refs = Array.isArray(ref) ? ref : [ref]

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
      if (this.reducedMotion) return

      await this.waitForRefs()

      const element = this.refs.backgroundBlur
      if (!element) return

      await AnimationPresets.fadeIn(element, {
        duration: options.duration ?? 500,
        delay: options.delay ?? 0
      })
    },

    /**
     * Fade out background blur
     */
    fadeOut: async (options: AnimationOptions = {}): Promise<void> => {
      if (this.reducedMotion) return

      await this.waitForRefs()

      const element = this.refs.backgroundBlur
      if (!element) return

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
    /**
     * Animate back button into view
     */
    show: async (): Promise<void> => {
      // Guard: Already shown or showing
      if (
        this.backButtonState === "shown" ||
        this.backButtonState === "showing"
      ) {
        return
      }

      // Guard: Reduced motion - just set state
      if (this.reducedMotion) {
        this.backButtonState = "shown"
        return
      }

      await this.waitForRefs()

      const element = this.refs.backButton
      if (!element) {
        console.warn("[AnimationController] backButton ref not found")
        return
      }

      // Cancel any in-flight animation
      if (this.backButtonAnimation) {
        this.backButtonAnimation.cancel()
        this.backButtonAnimation = null
      }

      this.backButtonState = "showing"

      this.backButtonAnimation = element.animate(
        [
          { width: "0", margin: "0" },
          { width: "60%", margin: "0 1rem 0 0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      )

      await this.backButtonAnimation.finished
        .then(() => {
          this.backButtonState = "shown"
          this.backButtonAnimation = null
        })
        .catch((err) => {
          // Animation was cancelled
          if (err.name === "AbortError") {
            // Silently handle cancellation
          }
          this.backButtonAnimation = null
        })
    },

    /**
     * Animate back button out of view
     */
    hide: async (): Promise<void> => {
      // Guard: Already hidden or hiding
      if (
        this.backButtonState === "hidden" ||
        this.backButtonState === "hiding"
      ) {
        return
      }

      // Guard: Reduced motion
      if (this.reducedMotion) {
        this.backButtonState = "hidden"
        return
      }

      await this.waitForRefs()

      const element = this.refs.backButton
      if (!element) return

      // Cancel any in-flight animation
      if (this.backButtonAnimation) {
        this.backButtonAnimation.cancel()
        this.backButtonAnimation = null
      }

      this.backButtonState = "hiding"

      this.backButtonAnimation = element.animate(
        [
          { width: "60%", margin: "0 1rem 0 0" },
          { width: "0", margin: "0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      )

      await this.backButtonAnimation.finished
        .then(() => {
          this.backButtonState = "hidden"
          this.backButtonAnimation = null
        })
        .catch((err) => {
          if (err.name === "AbortError") {
            // Silently handle cancellation
          }
          this.backButtonAnimation = null
        })
    }
  }

  // ============================================================================
  // Skip Button
  // ============================================================================

  skipButton = {
    /**
     * Animate skip button into view
     */
    show: async (): Promise<void> => {
      // Guard: Already shown or showing
      if (
        this.skipButtonState === "shown" ||
        this.skipButtonState === "showing"
      ) {
        return
      }

      // Guard: Reduced motion - just set state
      if (this.reducedMotion) {
        this.skipButtonState = "shown"
        return
      }

      await this.waitForRefs()

      const element = this.refs.skipButton
      if (!element) {
        console.warn("[AnimationController] skipButton ref not found")
        return
      }

      // Cancel any in-flight animation
      if (this.skipButtonAnimation) {
        this.skipButtonAnimation.cancel()
        this.skipButtonAnimation = null
      }

      this.skipButtonState = "showing"

      this.skipButtonAnimation = element.animate(
        [
          { width: "0", margin: "0" },
          { width: "40%", margin: "0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      )

      await this.skipButtonAnimation.finished
        .then(() => {
          this.skipButtonState = "shown"
          this.skipButtonAnimation = null
        })
        .catch((err) => {
          // Animation was cancelled
          if (err.name === "AbortError") {
            // Silently handle cancellation
          }
          this.skipButtonAnimation = null
        })
    },

    /**
     * Animate skip button out of view
     */
    hide: async (): Promise<void> => {
      // Guard: Already hidden or hiding
      if (
        this.skipButtonState === "hidden" ||
        this.skipButtonState === "hiding"
      ) {
        return
      }

      // Guard: Reduced motion
      if (this.reducedMotion) {
        this.skipButtonState = "hidden"
        return
      }

      await this.waitForRefs()

      const element = this.refs.skipButton
      if (!element) return

      // Cancel any in-flight animation
      if (this.skipButtonAnimation) {
        this.skipButtonAnimation.cancel()
        this.skipButtonAnimation = null
      }

      this.skipButtonState = "hiding"

      this.skipButtonAnimation = element.animate(
        [
          { width: "40%", margin: "0" },
          { width: "0", margin: "0" }
        ],
        {
          duration: 300,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1.275)",
          fill: "forwards"
        }
      )

      await this.skipButtonAnimation.finished
        .then(() => {
          this.skipButtonState = "hidden"
          this.skipButtonAnimation = null
        })
        .catch((err) => {
          if (err.name === "AbortError") {
            // Silently handle cancellation
          }
          this.skipButtonAnimation = null
        })
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
