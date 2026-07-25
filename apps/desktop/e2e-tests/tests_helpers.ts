export const getActualUrl = (url: string) => {
  const split = url.split("index.html#")
  const { pathname, search } = new URL(`http://bruh.gdlauncher.com${split[1]}`)
  return `${pathname}${search}`
}

/** The route alone, ignoring query state such as queued modals. */
export const getActualPath = (url: string) => {
  const split = url.split("index.html#")
  return new URL(`http://bruh.gdlauncher.com${split[1]}`).pathname
}
