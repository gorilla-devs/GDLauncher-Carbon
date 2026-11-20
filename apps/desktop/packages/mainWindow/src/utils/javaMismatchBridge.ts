// Event bridge for JAVA_MISMATCH error handling: a launch on a Java that
// doesn't fit the instance is held back so the crash help dialog can ask
// first. rspcClient runs outside the SolidJS tree, so it uses custom events.

export const JAVA_MISMATCH_EVENT = "gdl:java-mismatch"

/** As the core sends it with the JAVA_MISMATCH error. */
export interface JavaMismatchData {
  instanceId: number
  instanceName: string
  /** The Java Automatic would give the instance. */
  neededMajor: number
  usedMajor: number
  /** The Java the fix moves it to. */
  fixMajor: number
}

export function dispatchJavaMismatchEvent(data: JavaMismatchData) {
  window.dispatchEvent(new CustomEvent(JAVA_MISMATCH_EVENT, { detail: data }))
}

export function listenJavaMismatch(callback: (data: JavaMismatchData) => void) {
  const handler = (e: Event) => {
    callback((e as CustomEvent<JavaMismatchData>).detail)
  }
  window.addEventListener(JAVA_MISMATCH_EVENT, handler)
  return () => window.removeEventListener(JAVA_MISMATCH_EVENT, handler)
}
