import { marked } from "marked"

/** Refuse upstream error bodies before they reach the Markdown/render sink. */
export async function loadLegalDocument(url, fetchDocument = fetch) {
  const response = await fetchDocument(url)
  if (!response.ok) {
    throw new Error(`Unable to load legal document (HTTP ${response.status}).`)
  }
  return marked(await response.text())
}
