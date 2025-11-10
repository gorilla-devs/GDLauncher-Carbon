/**
 * AnimationTimeline - First-class animation sequencing system
 *
 * Provides a fluent API for building complex animation sequences
 * that can depend on async operations and conditional logic.
 *
 * @example
 * ```typescript
 * const timeline = new AnimationTimeline()
 *   .then(() => animations.sidebar.slideOut())
 *   .wait(100)
 *   .when(showSeasonal, async () => {
 *     await animations.seasonal.show(occasion);
 *     await timeline.wait(3000);
 *   })
 *   .parallel(
 *     () => animations.text.fadeIn('welcomeToText'),
 *     () => animations.backgroundBlur.fadeIn()
 *   )
 *   .then(() => navigate('/library'));
 *
 * await timeline.play();
 * ```
 */

type AnimationStep =
  | { type: "action"; fn: () => Promise<void> | void }
  | { type: "wait"; duration: number }
  | { type: "parallel"; actions: (() => Promise<void> | void)[] }
  | {
      type: "conditional"
      condition: boolean
      fn: () => Promise<void> | void
    }

export class AnimationTimeline {
  private steps: AnimationStep[] = []
  private cancelRequested = false

  /**
   * Execute an action in sequence
   * @param fn Function to execute (can be async)
   */
  then(fn: () => Promise<void> | void): this {
    this.steps.push({ type: "action", fn })
    return this
  }

  /**
   * Wait for a duration before continuing
   * @param duration Duration in milliseconds
   */
  wait(duration: number): this {
    this.steps.push({ type: "wait", duration })
    return this
  }

  /**
   * Execute multiple actions in parallel
   * All actions must complete before continuing to next step
   * @param actions Functions to execute in parallel
   */
  parallel(...actions: (() => Promise<void> | void)[]): this {
    this.steps.push({ type: "parallel", actions })
    return this
  }

  /**
   * Conditionally execute an action
   * @param condition If true, execute the action
   * @param fn Function to execute
   */
  when(condition: boolean, fn: () => Promise<void> | void): this {
    this.steps.push({ type: "conditional", condition, fn })
    return this
  }

  /**
   * Execute the timeline sequence
   * Returns a promise that resolves when all steps complete
   * Can be cancelled by calling cancel()
   */
  async play(): Promise<void> {
    this.cancelRequested = false

    for (const step of this.steps) {
      // Check for cancellation
      if (this.cancelRequested) {
        break
      }

      try {
        await this.executeStep(step)
      } catch (error) {
        console.error("[AnimationTimeline] Step failed:", error)
        // Continue to next step on error (animations shouldn't break flow)
      }
    }
  }

  /**
   * Cancel the timeline execution
   * Will stop after the current step completes
   */
  cancel(): void {
    this.cancelRequested = true
  }

  /**
   * Clear all steps (allows timeline reuse)
   */
  clear(): this {
    this.steps = []
    return this
  }

  /**
   * Get the number of steps in the timeline
   */
  get length(): number {
    return this.steps.length
  }

  // Private helper methods

  private async executeStep(step: AnimationStep): Promise<void> {
    switch (step.type) {
      case "action":
        await this.executeAction(step.fn)
        break

      case "wait":
        await this.executeWait(step.duration)
        break

      case "parallel":
        await this.executeParallel(step.actions)
        break

      case "conditional":
        if (step.condition) {
          await this.executeAction(step.fn)
        }
        break
    }
  }

  private async executeAction(fn: () => Promise<void> | void): Promise<void> {
    const result = fn()
    if (result instanceof Promise) {
      await result
    }
  }

  private executeWait(duration: number): Promise<void> {
    return new Promise((resolve) => {
      setTimeout(resolve, duration)
    })
  }

  private async executeParallel(
    actions: (() => Promise<void> | void)[]
  ): Promise<void> {
    await Promise.all(actions.map((action) => this.executeAction(action)))
  }
}

/**
 * Create a new animation timeline
 * Convenience function for creating timelines
 */
export function createTimeline(): AnimationTimeline {
  return new AnimationTimeline()
}

/**
 * Common animation presets as timeline factories
 */
export const AnimationPresets = {
  /**
   * Fade in element
   */
  fadeIn: (
    element: HTMLElement,
    options: { duration?: number; delay?: number } = {}
  ): Promise<void> => {
    const { duration = 300, delay = 0 } = options
    return element
      .animate([{ opacity: 0 }, { opacity: 1 }], {
        duration,
        delay,
        easing: "linear",
        fill: "forwards"
      })
      .finished.then(() => {})
  },

  /**
   * Fade out element
   */
  fadeOut: (
    element: HTMLElement,
    options: { duration?: number; delay?: number } = {}
  ): Promise<void> => {
    const { duration = 300, delay = 0 } = options
    return element
      .animate([{ opacity: 1 }, { opacity: 0 }], {
        duration,
        delay,
        easing: "linear",
        fill: "forwards"
      })
      .finished.then(() => {})
  },

  /**
   * Slide element horizontally
   */
  slideX: (
    element: HTMLElement,
    from: string,
    to: string,
    options: { duration?: number; delay?: number } = {}
  ): Promise<void> => {
    const { duration = 300, delay = 0 } = options
    return element
      .animate(
        [
          { transform: `translateX(${from})` },
          { transform: `translateX(${to})` }
        ],
        {
          duration,
          delay,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
          fill: "forwards"
        }
      )
      .finished.then(() => {})
  },

  /**
   * Scale element
   */
  scale: (
    element: HTMLElement,
    from: number,
    to: number,
    options = { duration: 300, delay: 0 }
  ): Promise<void> => {
    return element
      .animate(
        [{ transform: `scale(${from})` }, { transform: `scale(${to})` }],
        {
          duration: options.duration,
          delay: options.delay,
          easing: "cubic-bezier(0.175, 0.885, 0.32, 1)",
          fill: "forwards"
        }
      )
      .finished.then(() => {})
  }
}
