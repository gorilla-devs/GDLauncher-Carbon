import { describe, expect, it } from "vitest"
import { worldVersionCopy } from "./worldVersionCopy"
import content from "../../../../../../../packages/i18n/locale/english/content.json"
import tasks from "../../../../../../../packages/i18n/locale/english/tasks.json"
import { getTaskTranslationKey } from "../../../../../../../packages/i18n/helpers"
import { resolveTaskPoll } from "./hooks/resolveTaskPoll"

const translated = (key: string) =>
  content[key.replace("content:", "") as keyof typeof content]

describe("world version copy presentation", () => {
  it("renders API task IDs with an explicit retained-original copy task title", () => {
    const key = getTaskTranslationKey("InstanceTaskInstallWorldCopy")
    expect(key).toBe("tasks:_trn_instance_task_install_world_copy")
    const title = tasks[key.replace("tasks:", "") as keyof typeof tasks]
    expect(title).toContain("separate world copy {{world_name}}")
    expect(title).toContain("original save retained")
  })

  it("discloses the real explicit-version outcome and keeps its completion snapshot", () => {
    let row: string | undefined = "owned-world"
    const request = worldVersionCopy("world", 2, row)!
    expect(translated(request.action)).toBe("Install as separate save")
    expect(translated(request.disclosure)).toContain(
      "original save is retained"
    )
    expect(translated(request.disclosure)).toContain("additional disk space")
    row = undefined // scanner refresh must not change the pending request's message
    expect(worldVersionCopy("world", 2, row)).toBeUndefined()
    const completion = resolveTaskPoll(null, { type: "Known", value: 1 }, true)
    expect(completion.action).toEqual({
      kind: "completed",
      showSuccessToast: true
    })
    expect(translated(request.completion)).toBe(
      "Separate world save installed; original save retained"
    )
  })

  it.each([
    ["world", undefined, "owned-world", undefined],
    ["world", 2, undefined, undefined],
    ["mod", 2, "owned-mod", undefined],
    ["world", 2, "server-addon", 1]
  ] as const)(
    "keeps ordinary installs and server operations unchanged (%s, %s)",
    (kind, version, row, server) => {
      expect(worldVersionCopy(kind, version, row, server)).toBeUndefined()
    }
  )

  it("does not report successful copying after task failure", () => {
    const result = resolveTaskPoll(
      null,
      { type: "Failed", value: { cause: [], backtrace: "" } },
      true
    )
    expect(result.action).toEqual({
      kind: "completed",
      showSuccessToast: false
    })
  })
})
