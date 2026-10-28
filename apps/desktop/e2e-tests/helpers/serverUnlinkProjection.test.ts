import fs from "node:fs"
import { expect, it } from "vitest"
import type { ListServer } from "@gd/core_module/bindings"
import { computeServerLibraryItems } from "../../packages/mainWindow/src/pages/Library/utils/serverGrouping"

it.skipIf(!process.env.TASK43_SERVER_PROJECTION)(
  "actual reopened repository rows project in folders-first destination/source order",
  () => {
    const lines = fs
      .readFileSync(process.env.TASK43_SERVER_PROJECTION!, "utf8")
      .split("\n")
      .map((line) => line.split("\t"))
    const position = (value: string) =>
      value === "null" ? null : Number(value)
    const defaultGroupId = Number(lines.find((row) => row[0] === "D")![1])
    const groups = lines
      .filter((row) => row[0] === "G")
      .map((row) => ({
        id: Number(row[1]),
        libraryPosition: position(row[2]),
        name: row[3]
      }))
    const servers = lines
      .filter((row) => row[0] === "S")
      .map((row) => ({
        id: Number(row[1]),
        index: Number(row[2]),
        libraryPosition: position(row[3]),
        groupId: Number(row[4]),
        name: row[5]
      })) as ListServer[]
    const expectedIds = lines
      .filter((row) => row[0] === "E")
      .map((row) => row[1])
    expect(
      computeServerLibraryItems(servers, groups, "", defaultGroupId).map(
        (item) => item.id
      )
    ).toEqual(expectedIds)
  }
)
