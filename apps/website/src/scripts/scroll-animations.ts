/**
 * Scroll Animation System
 * Uses Motion One for smooth scroll-triggered animations
 */

import { animate, inView } from "motion"

type AnimationType = "fade-up" | "fade" | "slide-left" | "slide-right" | "scale"

interface AnimationConfig {
  initial: Record<string, string | number>
  animate: Record<string, string | number>
}

const ANIMATION_CONFIG: Record<AnimationType, AnimationConfig> = {
  "fade-up": {
    initial: { opacity: 0, transform: "translateY(30px)" },
    animate: { opacity: 1, transform: "translateY(0)" }
  },
  fade: {
    initial: { opacity: 0 },
    animate: { opacity: 1 }
  },
  "slide-left": {
    initial: { opacity: 0, transform: "translateX(-40px)" },
    animate: { opacity: 1, transform: "translateX(0)" }
  },
  "slide-right": {
    initial: { opacity: 0, transform: "translateX(40px)" },
    animate: { opacity: 1, transform: "translateX(0)" }
  },
  scale: {
    initial: { opacity: 0, transform: "scale(0.9)" },
    animate: { opacity: 1, transform: "scale(1)" }
  }
}

// Track cleanup functions for View Transitions
const cleanupFns: Array<() => void> = []

function initScrollAnimations() {
  // Clean up previous observers (for View Transitions)
  cleanupFns.forEach((fn) => fn())
  cleanupFns.length = 0

  document.querySelectorAll<HTMLElement>("[data-scroll]").forEach((el) => {
    const type = el.dataset.scroll as AnimationType
    const config = ANIMATION_CONFIG[type]

    if (!config) return

    // Set initial state immediately
    Object.assign(el.style, config.initial)

    // Animate when element enters viewport
    const cleanup = inView(
      el,
      () => {
        const delay = parseFloat(el.dataset.scrollDelay || "0") / 1000

        animate(el, config.animate, {
          duration: 0.6,
          easing: [0.16, 1, 0.3, 1], // ease-out-expo
          delay
        })
      },
      { margin: "-50px" }
    )

    cleanupFns.push(cleanup)
  })
}

// Initialize on Astro page load (handles View Transitions)
document.addEventListener("astro:page-load", initScrollAnimations, {
  once: false
})
