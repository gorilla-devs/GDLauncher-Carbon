/** On <html> while only the page slides: the rest of the window stays as it
 *  is (index.html). */
export const PAGE_SLIDE_CLASS = "gdl-page-slide"

const SLIDE_CLASSES = ["slide-right-transition", "slide-left-transition"]

/**
 * Marks a page slide for the view transition: `transitionClass` goes on the
 * layout's page box (`pageId`), or on the body where there is none, as
 * before. The returned function, called when the transition finishes, takes
 * the marks off, unless a newer slide has started since: that one's marks
 * stay until it finishes.
 */
export function createPageSlides(doc: Document, pageId: string) {
  let latest = 0
  return (transitionClass: string | undefined): (() => void) => {
    const mine = ++latest
    const page = doc.getElementById(pageId)
    const target = page ?? doc.body
    // A slide still running gives way to this one.
    doc.body.classList.remove(...SLIDE_CLASSES)
    page?.classList.remove(...SLIDE_CLASSES)
    doc.documentElement.classList.remove(PAGE_SLIDE_CLASS)
    if (transitionClass) {
      target.classList.add(transitionClass)
      if (page) doc.documentElement.classList.add(PAGE_SLIDE_CLASS)
    }
    return () => {
      if (mine !== latest || !transitionClass) return
      target.classList.remove(transitionClass)
      doc.documentElement.classList.remove(PAGE_SLIDE_CLASS)
    }
  }
}
