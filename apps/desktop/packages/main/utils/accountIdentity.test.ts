import { it, expect, vi } from "vitest"
import {
  AccountIdentityReceiver,
  CoreOutputLines,
  redactCoreIdentity
} from "./accountIdentity"
import { hashEmailForOverwolf } from "./emailHash"
it("rejects stale identity before immediate and deferred SDK handoff", () => {
  for (const ready of [false, true]) {
    const sink = vi.fn()
    const receiver = new AccountIdentityReceiver(sink)
    receiver.beginCore("core-one")
    receiver.setReady(ready)
    receiver.acceptPayload(
      JSON.stringify({
        session: "core-one",
        revision: 2,
        selected: null,
        email: null
      })
    )
    sink.mockClear()
    receiver.acceptPayload(
      JSON.stringify({
        session: "core-one",
        revision: 1,
        selected: "a",
        email: "a@example.invalid"
      })
    )
    expect(sink).not.toHaveBeenCalled()
    receiver.setReady(true)
    expect(sink).toHaveBeenLastCalledWith({})
  }
})
it("preserves current hashing and redacts all structured personal data", () => {
  const sink = vi.fn()
  const receiver = new AccountIdentityReceiver(sink)
  receiver.beginCore("core-one")
  receiver.setReady(true)
  const event = {
    session: "core-one",
    revision: 1,
    selected: "a",
    email: "a@example.invalid"
  }
  receiver.accept(event)
  expect(sink).toHaveBeenLastCalledWith(hashEmailForOverwolf(event.email))
  expect(
    redactCoreIdentity(`_GDL_ACCOUNT_EMAIL_:${JSON.stringify(event)}\nother`)
  ).toBe("_GDL_ACCOUNT_EMAIL_:<redacted>\nother")
})

it("binds resets only to the core lifecycle and refuses stale failures", () => {
  const sink = vi.fn()
  const receiver = new AccountIdentityReceiver(sink)
  receiver.beginCore("new-core")
  receiver.setReady(true)
  const current = {
    session: "new-core",
    revision: 3,
    selected: "b",
    email: "b@example.invalid"
  }
  receiver.accept(current)
  sink.mockClear()
  receiver.accept({
    session: "old-core",
    revision: 99,
    selected: "a",
    email: "a@example.invalid"
  })
  receiver.accept({
    session: "new-core",
    revision: 2,
    selected: "a",
    email: null
  })
  receiver.accept(current)
  expect(sink).not.toHaveBeenCalled()
  receiver.setReady(true)
  expect(sink).toHaveBeenLastCalledWith(hashEmailForOverwolf(current.email))
})
it("frames and redacts structured identity split across stdout chunks", () => {
  const frame = new CoreOutputLines()
  expect(frame.push('_GDL_ACCOUNT_EMAIL_:{"email":"a@exam')).toEqual([])
  const lines = frame.push('ple.invalid","selected":"a"}\nnext\n')
  expect(lines.map(redactCoreIdentity)).toEqual([
    "_GDL_ACCOUNT_EMAIL_:<redacted>",
    "next"
  ])
})
