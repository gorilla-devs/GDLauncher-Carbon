import { describe, expect, test } from "vitest"
import type { FEJavaIssue, ListInstance } from "@gd/core_module/bindings"
import {
  createJavaIssuePrompts,
  javaIssuesToShow,
  javaMismatchPrompt
} from "./javaIssues"

const instance = (
  id: number,
  name: string,
  issue: FEJavaIssue | null
): ListInstance => ({
  id,
  group_id: 1,
  index: 0,
  library_position: null,
  name,
  favorite: false,
  status: {
    status: "valid",
    value: {
      mc_version: "1.20.1",
      modloader: "fabric",
      modloader_version: "0.16.0",
      modpack: null,
      state: { state: "inactive", value: { failed_task: null } },
      java_issue: issue
    }
  },
  icon_revision: null,
  last_played: null,
  date_created: "2026-11-01T00:00:00Z",
  date_updated: "2026-11-01T00:00:00Z",
  seconds_played: 0,
  locked: false
})

const broken = (id: number): ListInstance => ({
  ...instance(id, "Broken", null),
  status: { status: "invalid", value: "JsonMissing" }
})

const on17 = { neededMajor: 21, usedMajor: 17, fixMajor: 21 }

describe("javaIssuesToShow", () => {
  test("lists only instances with a Java issue", () => {
    expect(
      javaIssuesToShow(
        [instance(1, "Fine", null), instance(2, "Crashy", on17), broken(3)],
        new Map()
      )
    ).toEqual([
      {
        instanceId: 2,
        instanceName: "Crashy",
        neededMajor: 21,
        usedMajor: 17,
        fixMajor: 21
      }
    ])
  })

  test("leaves out the ones already shown", () => {
    expect(
      javaIssuesToShow(
        [instance(2, "Crashy", on17)],
        new Map([[2, new Set([21])]])
      )
    ).toEqual([])
  })
})

describe("createJavaIssuePrompts", () => {
  test("opens each crash once, however often the list refetches", () => {
    const prompts = createJavaIssuePrompts()
    const crashed = [instance(2, "Crashy", on17)]

    expect(prompts.take(crashed)).toHaveLength(1)
    expect(prompts.take(crashed)).toEqual([])
  })

  test("the same crash after another launch is a new issue", () => {
    const prompts = createJavaIssuePrompts()
    const crashed = [instance(2, "Crashy", on17)]
    prompts.take(crashed)

    // The next launch clears the issue, and the game crashes the same way.
    expect(prompts.take([instance(2, "Crashy", null)])).toEqual([])
    expect(prompts.take(crashed)).toHaveLength(1)
  })

  test("another instance's launch doesn't bring back this one's crash", () => {
    const prompts = createJavaIssuePrompts()
    prompts.take([instance(2, "Crashy", on17), instance(5, "Other", on17)])

    expect(
      prompts.take([instance(2, "Crashy", on17), instance(5, "Other", null)])
    ).toEqual([])
  })

  test("a crash needing another Java is a new issue", () => {
    const prompts = createJavaIssuePrompts()
    prompts.take([instance(2, "Crashy", on17)])

    expect(
      prompts.take([
        instance(2, "Crashy", { neededMajor: 22, usedMajor: 21, fixMajor: 22 }),
        instance(5, "Other", on17)
      ])
    ).toEqual([
      {
        instanceId: 2,
        instanceName: "Crashy",
        neededMajor: 22,
        usedMajor: 21,
        fixMajor: 22
      },
      {
        instanceId: 5,
        instanceName: "Other",
        neededMajor: 21,
        usedMajor: 17,
        fixMajor: 21
      }
    ])
  })
})

describe("javaMismatchPrompt", () => {
  test("asks before the launch with what the core sent", () => {
    expect(
      javaMismatchPrompt({
        instanceId: 3,
        instanceName: "Survival",
        neededMajor: 21,
        usedMajor: 17,
        fixMajor: 21
      })
    ).toEqual({
      instanceId: 3,
      instanceName: "Survival",
      neededMajor: 21,
      usedMajor: 17,
      fixMajor: 21,
      beforeLaunch: true
    })
  })
})
