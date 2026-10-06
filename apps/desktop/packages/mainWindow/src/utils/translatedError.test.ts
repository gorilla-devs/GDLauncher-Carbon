import { expect, it } from "vitest"
import type { TypedTFunction } from "@gd/i18n"
import { TranslatedError, errorDescription } from "./translatedError"

const t = ((key: string) => `t(${key})`) as unknown as TypedTFunction

it("describes a failed lookup by the core's sentence, not the rspc payload", () => {
  const payload = JSON.stringify({
    code: 500,
    message: JSON.stringify({
      cause: [{ display: "Modrinth could not be reached", debug: "Reqwest" }],
      backtrace: "   0: carbon_app::managers::modplatforms"
    })
  })
  const text = errorDescription(
    t,
    new Error(payload),
    "notifications:_trn_pack_version_retry"
  )
  expect(text).toBe("Modrinth could not be reached")
  expect(text).not.toContain("{")
})

it("keeps the translated reason and the fallback", () => {
  expect(
    errorDescription(
      t,
      new TranslatedError("notifications:_trn_pack_version_retry", "English"),
      "notifications:_trn_pack_version_failed"
    )
  ).toBe("t(notifications:_trn_pack_version_retry)")
  expect(
    errorDescription(t, new Error(""), "notifications:_trn_pack_version_retry")
  ).toBe("t(notifications:_trn_pack_version_retry)")
  expect(
    errorDescription(t, "not an error", "notifications:_trn_pack_version_retry")
  ).toBe("t(notifications:_trn_pack_version_retry)")
})
