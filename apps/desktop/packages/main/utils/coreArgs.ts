export interface CoreArgsOptions {
  runtimePath: string
  baseApi?: string | null
  e2eAuthBase?: string | null
  e2eEntitlementKey?: string | null
}

/** Same as `EXIT_WITH_LAUNCHER_ARG` in crates/carbon_app/src/launcher.rs. */
export const EXIT_WITH_LAUNCHER_ARG = "--exit_with_launcher"

/**
 * The argv handed to the core module.
 *
 * The e2e overrides are forwarded unconditionally; a core module built without
 * the `e2e` cargo feature ignores them, which keeps the gate in exactly one
 * place instead of splitting it across two build systems.
 */
export function buildCoreModuleArgs(opts: CoreArgsOptions): string[] {
  const args = ["--runtime_path", opts.runtimePath]

  if (opts.baseApi) {
    args.push("--base_api", opts.baseApi)
  }

  if (opts.e2eAuthBase) {
    args.push("--e2e_auth_base", opts.e2eAuthBase)
  }

  if (opts.e2eEntitlementKey) {
    args.push("--e2e_entitlement_key", opts.e2eEntitlementKey)
  }

  // The core watches the pipe this process holds to its standard input, and
  // shuts down when this process is gone (a crash or a kill included): on
  // unix nothing else would end it. Last, so it is never taken for the value
  // of an argument before it.
  args.push(EXIT_WITH_LAUNCHER_ARG)

  return args
}
