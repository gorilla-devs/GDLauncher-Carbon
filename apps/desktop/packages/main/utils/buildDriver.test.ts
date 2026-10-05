import { it, expect } from "vitest"
import fs from "node:fs"
import vm from "node:vm"
import path from "node:path"
import { runBuild } from "../../../../../scripts/build.mjs"
const quiet = { log() {} }
it("Task23 public switch sets both halves in all ordered target plans", () => {
  for (const target of ["linux-x64", "win-x64", "mac-universal"])
    for (const input of [
      undefined,
      "",
      "0",
      "false",
      "FALSE",
      "true",
      "1",
      "enabled"
    ]) {
      const calls: { command: string; define: string | undefined }[] = []
      runBuild(
        target,
        { GDL_E2E: input, GDL_E2E_BUILD: "true" },
        (command: string, options: any) => {
          calls.push({ command, define: options.env.GDL_E2E_BUILD })
          return { status: 0 }
        },
        quiet
      )
      const enabled = ["true", "1", "enabled"].includes(input ?? "")
      expect(calls.at(-1)?.define).toBe(String(enabled))
      expect(
        calls
          .filter((c) => c.command.startsWith("cargo"))
          .every((c) => c.command.includes("--features e2e") === enabled)
      ).toBe(true)
      if (target === "mac-universal")
        expect(calls.slice(0, 4).map((c) => c.command)).toEqual([
          `cargo build -p carbon_app --release${enabled ? " --features e2e" : ""} --target aarch64-apple-darwin`,
          "pnpm run prepare-release:mac-arm64",
          `cargo build -p carbon_app --release${enabled ? " --features e2e" : ""} --target x86_64-apple-darwin`,
          "pnpm run prepare-release:mac-x64"
        ])
    }
  let define = ""
  runBuild(
    "win-x64",
    {},
    (_: string, opts: any) => {
      define = opts.env.GDL_E2E_BUILD
      return { status: 0 }
    },
    quiet
  )
  expect(define).toBe("false")
})
it("Task23 injected driver rejects invalid targets and signal/status/spawn failures", () => {
  for (const target of [undefined, "invalid"])
    expect(() =>
      runBuild(
        target,
        {},
        () => {
          throw Error("must not spawn")
        },
        quiet
      )
    ).toThrow("build target")
  for (const result of [
    { status: null },
    { status: 7 },
    { error: Error("failed to start") }
  ]) {
    let calls = 0
    expect(() =>
      runBuild(
        "linux-x64",
        {},
        () => {
          calls++
          return result
        },
        quiet
      )
    ).toThrow()
    expect(calls).toBe(1)
  }
})
it("Task23 actual production cross-env forwards false despite inherited public/internal true", () => {
  const root = path.resolve(__dirname, "../../../../.."),
    scripts = JSON.parse(
      fs.readFileSync(path.join(root, "package.json"), "utf8")
    ).scripts
  const source = fs
    .readFileSync(
      path.join(root, "node_modules/cross-env/dist/index.js"),
      "utf8"
    )
    .replace(/^import .*;\n/gm, "")
    .replace("export function crossEnv", "function crossEnv")
  for (const target of ["linux-x64", "win-x64", "mac-universal"]) {
    const command = scripts[`build:${target}-prod`].split(" && ").at(-1)
    let forwarded = ""
    const context = {
      process: {
        env: { GDL_E2E: "true", GDL_E2E_BUILD: "true" },
        platform: "linux",
        on() {}
      },
      spawn: (_: string, __: string[], opts: any) => {
        forwarded = opts.env.GDL_E2E_BUILD
        return { on() {} }
      },
      invariant: (ok: boolean) => {
        if (!ok) throw Error("cross-env invariant")
      },
      commandConvert: (s: string) => s,
      varValueConvert: (s: string) => s
    }
    vm.runInNewContext(
      source +
        "\ncrossEnv(" +
        JSON.stringify(command.split(" ").slice(1)) +
        ");",
      context
    )
    expect(forwarded).toBe("false")
  }
})
