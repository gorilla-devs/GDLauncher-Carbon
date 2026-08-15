import { describe, test, expect } from "vitest"
import { ListServer } from "@gd/core_module/bindings"
import { computeServerVirtualGroups } from "../pages/Library/utils/serverGrouping"

// The servers tab renders virtual groups whenever the library is in accordion
// mode (instancesGroupBy set) or a search filter is active. Groups that end up
// with zero tiles are hidden by GroupSection, so every matching server must be
// placed inside its group or the tab renders nothing.

let nextId = 1

function makeServer(overrides: Partial<ListServer> = {}): ListServer {
  const id = nextId++
  return {
    id,
    groupId: 1,
    index: id,
    libraryPosition: null,
    name: `Server ${id}`,
    favorite: false,
    serverType: "vanilla",
    gameVersion: "1.21.4",
    port: 25565,
    dateCreated: "2026-07-01T00:00:00Z",
    lastStarted: null,
    state: { status: "stopped", failed_task: null },
    iconRevision: null,
    modloaderType: null,
    modloaderVersion: null,
    modpackInfo: null,
    autoRestartAbandoned: false,
    ...overrides
  }
}

describe("computeServerVirtualGroups", () => {
  test("accordion mode places every server inside its game version group", () => {
    const servers = [
      makeServer({ name: "Alpha", gameVersion: "1.21.4" }),
      makeServer({ name: "Beta", gameVersion: "1.21.4" }),
      makeServer({ name: "Gamma", gameVersion: "1.20.1" })
    ]

    const groups = computeServerVirtualGroups(
      servers,
      { instancesGroupBy: "gameVersion" },
      "",
      "Search Results"
    )

    expect(groups.map((g) => g.name)).toEqual(["1.20.1", "1.21.4"])
    expect(
      groups.flatMap((g) => g.instances.map((s) => s.name)).sort()
    ).toEqual(["Alpha", "Beta", "Gamma"])
    const v1214 = groups.find((g) => g.name === "1.21.4")!
    expect(v1214.instances.map((s) => s.name)).toEqual(["Alpha", "Beta"])
  })

  test("instance-only groupBy values fall back to game version grouping with servers included", () => {
    // instancesGroupBy is shared with the instances library, so it can hold
    // values that only make sense for instances (modloader, modplatform).
    const servers = [makeServer({ gameVersion: "1.21.4" })]

    const groups = computeServerVirtualGroups(
      servers,
      { instancesGroupBy: "modloader" },
      "",
      "Search Results"
    )

    expect(groups).toHaveLength(1)
    expect(groups[0].instances).toHaveLength(1)
  })

  test("flat search returns matching servers inside the results group", () => {
    const servers = [
      makeServer({ name: "Survival World" }),
      makeServer({ name: "Creative Hub" })
    ]

    const groups = computeServerVirtualGroups(
      servers,
      { instancesGroupBy: null },
      "survival",
      "Search Results"
    )

    expect(groups).toHaveLength(1)
    expect(groups[0].instances.map((s) => s.name)).toEqual(["Survival World"])
  })

  test("flat search with no matches returns no groups", () => {
    const servers = [makeServer({ name: "Survival World" })]

    const groups = computeServerVirtualGroups(
      servers,
      { instancesGroupBy: null },
      "nomatch",
      "Search Results"
    )

    expect(groups).toEqual([])
  })

  test("name filter applies in accordion mode and hides empty groups", () => {
    const servers = [
      makeServer({ name: "Survival World", gameVersion: "1.21.4" }),
      makeServer({ name: "Creative Hub", gameVersion: "1.20.1" })
    ]

    const groups = computeServerVirtualGroups(
      servers,
      { instancesGroupBy: "gameVersion" },
      "survival",
      "Search Results"
    )

    expect(groups.map((g) => g.name)).toEqual(["1.21.4"])
    expect(groups[0].instances.map((s) => s.name)).toEqual(["Survival World"])
  })

  test("sorts servers within a group by the shared sort setting", () => {
    const servers = [
      makeServer({
        name: "Old",
        dateCreated: "2026-01-01T00:00:00Z",
        gameVersion: "1.21.4"
      }),
      makeServer({
        name: "New",
        dateCreated: "2026-06-01T00:00:00Z",
        gameVersion: "1.21.4"
      })
    ]

    const groups = computeServerVirtualGroups(
      servers,
      {
        instancesGroupBy: "gameVersion",
        instancesSortBy: "created",
        instancesSortByAsc: false
      },
      "",
      "Search Results"
    )

    expect(groups[0].instances.map((s) => s.name)).toEqual(["New", "Old"])
  })
})
