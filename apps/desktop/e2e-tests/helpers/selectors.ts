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
  instancePlay: "instance-play",
  instanceContextDelete: "instance-context-delete",
  confirmInstanceDeletion: "confirm-instance-deletion",
  // Generic close (X) rendered by ModalLayout's header — present on every
  // modal that doesn't opt out with `noHeader`. Used here to dismiss the
  // changelogs modal on a fresh runtime path.
  modalClose: "modal-close",
  // The onboarding wizard has no header (`noHeader: true`), so its own
  // dismissal path is stepping through it: two "Next" clicks (the same
  // testid on both, since only one step is ever mounted at a time) into
  // the final step's "Skip" control.
  onboardingNext: "onboarding-next",
  onboardingSkip: "onboarding-skip"
})

export function byTestId(id: string): string {
  return `[data-testid="${id}"]`
}

/** An instance tile located by the name the test gave it. */
export function byInstanceName(name: string): string {
  return `[data-testid="instance-tile"][data-instance-name="${name}"]`
}
