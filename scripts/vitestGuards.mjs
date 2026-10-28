import fg from "fast-glob"
import { readFileSync, statSync } from "node:fs"
import { homedir } from "node:os"
import path from "node:path"

/**
 * Checks the vitest configs of apps/desktop and apps/website run when they
 * load, so a setup mistake stops the run with a plain message instead of
 * showing up as a red (or silently missing) test on CI.
 */

/**
 * A file claimed by two projects runs twice, in two environments, and only
 * one of them is the one it was written for.
 * @param {string} root directory the include globs are relative to
 * @param {Array<{ name: string, include: string[] }>} projects
 * @param {string[]} ignore
 */
export function assertOneProjectPerFile(root, projects, ignore) {
  const owners = new Map()
  for (const { name, include } of projects) {
    for (const file of fg.sync(include, { cwd: root, ignore })) {
      owners.set(file, [...(owners.get(file) ?? []), name])
    }
  }
  const claimedTwice = [...owners].filter(([, names]) => names.length > 1)
  if (claimedTwice.length > 0) {
    throw new Error(
      "Each unit test file must belong to one vitest project. Remove these from all but one project's include:\n" +
        claimedTwice
          .map(([file, names]) => `  ${file}: ${names.join(", ")}`)
          .join("\n")
    )
  }
}

/**
 * A `vitest.*.config.mjs` that `test:unit` does not load is how the fix
 * round's tests ended up run by nothing on CI.
 * @param {string} root
 * @param {string[]} wired configs the default config runs as projects
 * @param {Record<string, string>} standalone config -> the command that runs it
 */
export function assertEveryConfigRuns(root, wired, standalone) {
  const stray = fg
    .sync("vitest.*.config.mjs", { cwd: root })
    .filter((file) => !wired.includes(file) && !(file in standalone))
  if (stray.length > 0) {
    throw new Error(
      `No script runs ${stray.join(", ")}, so CI never runs its tests. ` +
        "Add it to the projects in the vitest.config.mjs next to it (or move its tests into an existing project and delete it). " +
        "If it must run on its own, add it, with the command that runs it, to the standalone list that vitest.config.mjs passes to assertEveryConfigRuns, and call that command from a script CI runs."
    )
  }
}

function machineSpecificPrefixes() {
  const home = homedir().replaceAll("\\", "/").replace(/\/+$/, "")
  // "/" or "/root" is too short to tell a real home from a synthetic fixture
  // path; a user home ("/home/runner", "C:\Users\me") is not.
  const prefixes =
    home.split("/").filter(Boolean).length >= 2
      ? [
          `${home}/`,
          `${home.replaceAll("/", "\\")}\\`,
          `${home.replaceAll("/", "\\\\")}\\\\`
        ]
      : []
  // The agent and review scratch area is never checked in.
  return [...prefixes, "/.claude/", "\\.claude\\"]
}

/**
 * A test that names a path under this machine's home directory, or under
 * the untracked `.claude/` scratch area, passes here and fails on every CI
 * runner and every other checkout.
 * @param {string} root
 * @param {string[]} patterns files to scan, relative to root
 * @param {string[]} ignore
 */
export function assertNoMachinePaths(root, patterns, ignore) {
  const prefixes = machineSpecificPrefixes()
  const hits = []
  for (const file of fg.sync(patterns, { cwd: root, ignore })) {
    const lines = readFileSync(path.join(root, file), "utf8").split("\n")
    lines.forEach((line, index) => {
      const prefix = prefixes.find((candidate) => line.includes(candidate))
      if (prefix) hits.push(`  ${file}:${index + 1} names ${prefix}`)
    })
  }
  if (hits.length > 0) {
    throw new Error(
      "These tests name a path that exists only on this machine, so they fail on CI and in every other checkout:\n" +
        hits.join("\n") +
        "\nCreate temporary files under os.tmpdir() and resolve repository files from import.meta.url instead."
    )
  }
}

/**
 * Tests that load a workspace package from its built `dist` run against
 * whatever was built last. When the sources changed since, they fail with
 * errors that look like broken mocks, or pass against strings the sources no
 * longer have.
 * @param {string} root
 * @param {Array<{ output: string, sources: string[], ignore?: string[] }>} builds
 *   paths relative to root
 * @param {string} rebuild command that rebuilds every package listed
 */
export function assertBuiltAfterSources(root, builds, rebuild) {
  const problems = []
  for (const { output, sources, ignore = [] } of builds) {
    const built = statSync(path.join(root, output), { throwIfNoEntry: false })
    if (!built) {
      problems.push(`  ${output} does not exist`)
      continue
    }
    const newer = fg
      .sync(sources, { cwd: root, ignore, stats: true })
      .filter((entry) => entry.stats.mtimeMs > built.mtimeMs)
      .map((entry) => entry.path)
    if (newer.length > 0) {
      problems.push(
        `  ${output} is older than ${newer.slice(0, 3).join(", ")}` +
          (newer.length > 3 ? ` and ${newer.length - 3} more` : "")
      )
    }
  }
  if (problems.length > 0) {
    throw new Error(
      "The tests would run against an outdated build of a workspace package:\n" +
        problems.join("\n") +
        `\nRebuild with \`${rebuild}\` from the repository root, or run \`pnpm test:unit\` there, which rebuilds first.`
    )
  }
}
