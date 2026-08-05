/** `AddonType`'s serialised form, as it reaches the frontend on `Mod.addon_type`. */
const WORLDS = "worlds"

/**
 * Whether an addon type can be meaningfully enabled/disabled.
 *
 * Disabling works by appending `.disabled` to the name inside the type's
 * folder. That is coherent for a file and incoherent for a world: a save is a
 * directory, Minecraft still lists a renamed one, and our own scanner
 * re-reports `<name>.disabled` as a *new, enabled* world. The control cannot do
 * what it claims for worlds, so it is not offered.
 */
export const supportsEnableToggle = (addonType: string): boolean =>
  addonType !== WORLDS

/**
 * Whether deleting this addon type should ask first.
 *
 * Only worlds. Deleting a world destroys save data that cannot be recovered,
 * where every other addon type can simply be reinstalled — confirming all of
 * them would train people to click through the one that matters.
 */
export const requiresDeletionConfirmation = (addonType: string): boolean =>
  addonType === WORLDS
