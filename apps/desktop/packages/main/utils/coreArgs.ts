export interface CoreArgsOptions {
  runtimePath: string
  baseApi?: string | null
  e2eAuthBase?: string | null
  e2eEntitlementKey?: string | null
}

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

  return args
}
