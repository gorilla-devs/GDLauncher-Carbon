import { describe, expect, it } from "vitest"
import i18next from "i18next"
import type { FEGameCrash } from "@gd/core_module/bindings"
import english from "../../../../../../../packages/i18n/locale/english/notifications.json"
import { describeGameCrash, type Translate } from "./describe"

describe("describeGameCrash", () => {
  it("says which game crashed or did not start, and the one thing known about it", async () => {
    const i18n = i18next.createInstance()
    await i18n.init({
      lng: "en",
      ns: ["notifications"],
      resources: { en: { notifications: english } }
    })
    const t: Translate = (key, options) => i18n.t(key as string, options)
    const crash = (kind: FEGameCrash["kind"]): FEGameCrash => ({
      id: 1,
      instance: 4,
      name: "Steve's Pack",
      kind
    })
    expect(
      describeGameCrash(crash({ type: "Crashed", value: { report: true } }), t)
    ).toEqual({
      title: "Steve's Pack crashed",
      description: "It saved a crash report."
    })
    expect(
      describeGameCrash(crash({ type: "Crashed", value: { report: false } }), t)
    ).toEqual({ title: "Steve's Pack crashed" })
    expect(
      describeGameCrash(
        crash({ type: "DidNotStart", value: { seconds: 3, code: 1 } }),
        t
      )
    ).toEqual({
      title: "Steve's Pack didn't start",
      description: "It closed after 3 seconds with exit code 1."
    })
    expect(
      describeGameCrash(
        crash({ type: "DidNotStart", value: { seconds: 1, code: null } }),
        t
      )
    ).toEqual({
      title: "Steve's Pack didn't start",
      description: "It closed after 1 second."
    })
  })
})
