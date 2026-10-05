import { it, expect } from "vitest"
import { createCoreOutput } from "../coreOutput"
it("complete production core records retain a noncorruption cause", () => {
  const outcomes: any[] = []
  const core = createCoreOutput({
    log: () => {},
    settle: (r) => outcomes.push(r),
    record: () => {},
    progress: () => {}
  })
  for (const byte of Buffer.from("_STATUS_:DB_MIGRATION_FAILED|access\r\n"))
    core.push(Buffer.from([byte]))
  expect(outcomes).toHaveLength(1)
  expect(outcomes[0]).toMatchObject({ type: "error", dbRecovery: "access" })
})

it.each([
  ["DB_MIGRATION_FAILED", "locked", "locked"],
  ["DB_MIGRATION_FAILED", "storage", "storage"],
  ["DB_MIGRATION_FAILED", "history", "version"],
  ["DB_MIGRATION_FAILED", undefined, "unknown"],
  ["DB_MIGRATION_FAILED", "unrecognized", "unknown"],
  ["DB_CORRUPT", undefined, "corruption"],
  ["DB_DIVERGED", undefined, "version"],
  ["DB_DOWNGRADE_FAILED", "C:\\synthetic\\snapshot.db", "downgrade"]
])(
  "%s / %s preserves its whitelisted cause and snapshot semantics",
  (event, payload, category) => {
    const outcomes: any[] = []
    const core = createCoreOutput({
      log: () => {},
      settle: (r) => outcomes.push(r),
      record: () => {},
      progress: () => {}
    })
    core.push(Buffer.from(`_STATUS_:${event}${payload ? "|" + payload : ""}\n`))
    expect(outcomes[0]).toMatchObject({ type: "error", dbRecovery: category })
    expect(outcomes[0].snapshotPath).toBe(
      event === "DB_DOWNGRADE_FAILED" ? payload : undefined
    )
  }
)
it("unrelated core error does not diagnose a database and backwards migration remains distinct", () => {
  const outcomes: any[] = []
  const core = createCoreOutput({
    log: () => {},
    settle: (r) => outcomes.push(r),
    record: () => {},
    progress: () => {}
  })
  core.fail("startup timeout")
  expect(outcomes[0].dbRecovery).toBeUndefined()
  const backwards = createCoreOutput({
    log: () => {},
    settle: (r) => outcomes.push(r),
    record: () => {},
    progress: () => {}
  })
  backwards.push(Buffer.from("_STATUS_:BACKWARDS_MIGRATION\n"))
  expect(outcomes[1].type).toBe("backwardsMigration")
})
