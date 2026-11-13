import { describe, expect, it } from "vitest"
import { deletionTarget } from "./deletionTarget"
import { deletionStopsServer, serverDeleteLabelKey } from "./serverDeletion"

describe("what deleting a server is said to do", () => {
  it("follows the Recycle Bin setting, and says nothing until it is known", () => {
    expect(deletionTarget(undefined)).toBeUndefined()
    expect(deletionTarget(true)).toBe("recycleBin")
    expect(deletionTarget(false)).toBe("permanent")
  })
  it("stops only a server this session is running", () => {
    expect(deletionStopsServer("running")).toBe(true)
    for (const status of [
      undefined,
      "stopped",
      "starting",
      "stopping",
      "installing",
      "deleting",
      "recoveredRunning",
      "recoveredUnverified"
    ]) {
      expect(deletionStopsServer(status), String(status)).toBe(false)
    }
  })
  it("labels the button with everything the click does", () => {
    expect(serverDeleteLabelKey(false, "recycleBin")).toBe(
      "instances:_trn_instance_confirm_deletion.delete"
    )
    expect(serverDeleteLabelKey(false, undefined)).toBe(
      "instances:_trn_instance_confirm_deletion.delete"
    )
    expect(serverDeleteLabelKey(true, "recycleBin")).toBe(
      "instances:_trn_server_delete_stop_and_delete"
    )
    expect(serverDeleteLabelKey(true, undefined)).toBe(
      "instances:_trn_server_delete_stop_and_delete"
    )
    expect(serverDeleteLabelKey(false, "permanent")).toBe(
      "instances:_trn_server_delete_permanently"
    )
    expect(serverDeleteLabelKey(true, "permanent")).toBe(
      "instances:_trn_server_delete_stop_and_delete_permanently"
    )
  })
})
