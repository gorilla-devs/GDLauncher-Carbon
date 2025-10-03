import { createSignal } from "solid-js"

export const [loggedOut, setLoggedOut] = createSignal(false)

export const libraryPathRegex = /^\/library\/(\w+)(\/\w+)*\/?$/

export const getInstanceIdFromPath = (path: string) => {
  const instaceUrlRegex = libraryPathRegex.exec(path)
  const instanceId = instaceUrlRegex?.[1]
  return instanceId?.replace("/", "")
}

export const isLibraryPath = (path: string) => {
  return libraryPathRegex.test(path)
}

export const isSearchPath = (path: string | number) => {
  return path.toString().startsWith("/search")
}

export const isAddonPath = (path: string | number) => {
  return path.toString().startsWith("/addon")
}

export const isNewsPath = (path: string | number) => {
  return path.toString().startsWith("/news")
}

export const isNewsDetailPath = (path: string | number) => {
  return /^\/news\/[^/]+$/.exec(path.toString())
}
