/** On <html> while a page slides: the rest of the window stays as it is
 *  (index.html). */
export const PAGE_SLIDE_CLASS = "gdl-page-slide"

const SLIDE_CLASSES = ["slide-right-transition", "slide-left-transition"]

/**
 * Marks a page slide for the view transition. The marks go on <html>, which
 * outlives the navigation, and index.html names the panel that slides from
 * them: the old page's before the navigation and the new page's after, though
 * they are different elements. The returned function, called when the
 * transition finishes, takes the marks off, unless a newer slide has started
 * since: that one's marks stay until it finishes.
 */
export function createPageSlides(doc: Document) {
  let latest = 0
  const root = doc.documentElement
  return (transitionClass: string | undefined): (() => void) => {
    const mine = ++latest
    // A slide still running gives way to this one.
    root.classList.remove(PAGE_SLIDE_CLASS, ...SLIDE_CLASSES)
    if (transitionClass) root.classList.add(PAGE_SLIDE_CLASS, transitionClass)
    return () => {
      if (mine !== latest || !transitionClass) return
      root.classList.remove(PAGE_SLIDE_CLASS, transitionClass)
    }
  }
}
