import { useTransContext } from "@gd/i18n"
import { formatSize } from "./javaText"

/** `formatSize` in the app's language: "189.4 MB", or "189,4 МБ". */
export function useFormatSize(): (bytes: number) => string {
  const [t, { getI18next }] = useTransContext()
  return (bytes) => formatSize(bytes, getI18next().language, (key) => t(key))
}
