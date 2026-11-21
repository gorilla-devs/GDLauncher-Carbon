// Theme hooks: `data-gdl` is a space-separated token list that themes select
// with `[data-gdl~="name"]`. A component adds its own token and keeps any token
// the app passes in (e.g. `library.search`).
export function gdlHooks(own: string, extra?: unknown): string {
  return own + (typeof extra === "string" && extra ? " " + extra : "")
}

/**
 * A hook with its state tokens: `base` plus `base.<state>` for every state
 * that is on, in the order given. `withStates("tab", { active: true })` is
 * `"tab tab.active"`.
 */
export function withStates(base: string, states: Record<string, unknown>) {
  let out = base
  for (const [name, on] of Object.entries(states)) {
    if (on) out += ` ${base}.${name}`
  }
  return out
}
