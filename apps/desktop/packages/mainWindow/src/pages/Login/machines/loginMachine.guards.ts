import type { LoginMachineContext } from "./loginMachine.types"

export const guards = {
  shouldShowSeasonalSplash: ({ context }: { context: LoginMachineContext }) =>
    context.currentOccasion !== null,

  isAddingFromSettings: ({ context }: { context: LoginMachineContext }) =>
    context.isAddingAccount,

  isAddingMicrosoftFromSettings: ({
    context
  }: {
    context: LoginMachineContext
  }) => context.isAddingMicrosoftAccount,

  isAddingGdlFromSettings: ({ context }: { context: LoginMachineContext }) =>
    context.isAddingGdlAccount,

  needsTermsAcceptance: ({ context }: { context: LoginMachineContext }) =>
    !context.termsAndPrivacyAccepted,

  hasExistingAccount: ({ context }: { context: LoginMachineContext }) =>
    context.activeUuid !== null,

  termsAccepted: ({ context }: { context: LoginMachineContext }) =>
    context.termsAccepted,

  foundExistingGDLAccount: ({ context }: { context: LoginMachineContext }) =>
    context.foundExistingGDLAccount && context.foundGDLAccountData !== null,

  hasLinkedGDLAccount: ({ context }: { context: LoginMachineContext }) =>
    context.gdlAccountId === context.activeUuid,

  previouslySkippedGDL: ({ context }: { context: LoginMachineContext }) =>
    context.gdlAccountId === "",

  shouldReturnToSettings: ({ context }: { context: LoginMachineContext }) =>
    (context.isAddingMicrosoftAccount || context.isAddingGdlAccount) &&
    context.returnPath !== null,

  shouldUseViewTransitions: ({ context }: { context: LoginMachineContext }) =>
    !context.reducedMotion &&
    typeof document !== "undefined" &&
    "startViewTransition" in document,

  accountExists: ({ context }: { context: LoginMachineContext }) =>
    context.accounts.some((acc) => acc.uuid === context.activeUuid)
}
