/**
 * The `data-testid` values the e2e suite drives.
 *
 * Kept in one place so renaming an anchor is a single edit rather than a
 * search through specs. Deliberately few: only the elements a test must click
 * or wait on carry one.
 */
export const TEST_IDS = Object.freeze({
  termsCheckbox: "login-terms-checkbox",
  welcomeContinue: "login-welcome-continue",
  termsContinue: "login-terms-continue",
  useDeviceCode: "login-use-device-code",
  deviceCode: "login-device-code",
  gdlSyncAccount: "login-gdl-sync-account",
  libraryRoot: "library-root",
  betaPromptNever: "beta-prompt-never",
  addInstance: "library-add-instance",
  instanceCreationCustomTab: "instance-creation-custom-tab",
  instanceCreationName: "instance-creation-name",
  instanceCreationVersionTrigger: "instance-creation-version-trigger",
  instanceCreationSubmit: "instance-creation-submit",
  instanceTile: "instance-tile",
  instancePlay: "instance-play"
})

export function byTestId(id: string): string {
  return `[data-testid="${id}"]`
}

/** An instance tile located by the name the test gave it. */
export function byInstanceName(name: string): string {
  return `[data-testid="instance-tile"][data-instance-name="${name}"]`
}
