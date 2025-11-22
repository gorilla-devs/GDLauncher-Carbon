import { describe, test, expect } from "vitest"
import { normalizeEmail } from "../emailHash"

describe("normalizeEmail", () => {
  describe("basic normalization", () => {
    test("trims leading and trailing whitespace", () => {
      expect(normalizeEmail("  test@example.com  ")).toBe("test@example.com")
      expect(normalizeEmail("\ttest@example.com\n")).toBe("test@example.com")
      expect(normalizeEmail("   test@example.com")).toBe("test@example.com")
      expect(normalizeEmail("test@example.com   ")).toBe("test@example.com")
    })

    test("converts to lowercase", () => {
      expect(normalizeEmail("TEST@EXAMPLE.COM")).toBe("test@example.com")
      expect(normalizeEmail("Test@Example.Com")).toBe("test@example.com")
      expect(normalizeEmail("TeSt@ExAmPlE.cOm")).toBe("test@example.com")
    })

    test("combines trimming and lowercasing", () => {
      expect(normalizeEmail("  TEST@EXAMPLE.COM  ")).toBe("test@example.com")
      expect(normalizeEmail("\t Test@Example.Com \n")).toBe("test@example.com")
    })
  })

  describe("Gmail-specific rules", () => {
    test("removes dots from Gmail local part", () => {
      expect(normalizeEmail("jane.doe@gmail.com")).toBe("janedoe@gmail.com")
      expect(normalizeEmail("j.a.n.e@gmail.com")).toBe("jane@gmail.com")
      expect(normalizeEmail("test.user.name@gmail.com")).toBe(
        "testusername@gmail.com"
      )
    })

    test("removes plus tags from Gmail addresses", () => {
      expect(normalizeEmail("jane+home@gmail.com")).toBe("jane@gmail.com")
      expect(normalizeEmail("test+spam@gmail.com")).toBe("test@gmail.com")
      expect(normalizeEmail("user+tag123@gmail.com")).toBe("user@gmail.com")
    })

    test("removes both dots and plus tags from Gmail", () => {
      expect(normalizeEmail("jane.doe+home@gmail.com")).toBe(
        "janedoe@gmail.com"
      )
      expect(normalizeEmail("test.user+work@gmail.com")).toBe(
        "testuser@gmail.com"
      )
      expect(normalizeEmail("j.a.n.e+tag@gmail.com")).toBe("jane@gmail.com")
    })

    test("handles Gmail with uppercase", () => {
      expect(normalizeEmail("Jane.Doe@Gmail.Com")).toBe("janedoe@gmail.com")
      expect(normalizeEmail("TEST.USER+TAG@GMAIL.COM")).toBe(
        "testuser@gmail.com"
      )
    })

    test("handles Gmail with whitespace", () => {
      expect(normalizeEmail("  jane.doe+home@gmail.com  ")).toBe(
        "janedoe@gmail.com"
      )
    })

    test("does NOT apply Gmail rules to non-Gmail addresses", () => {
      // Dots should be preserved for non-Gmail
      expect(normalizeEmail("jane.doe@example.com")).toBe(
        "jane.doe@example.com"
      )
      expect(normalizeEmail("test.user@yahoo.com")).toBe("test.user@yahoo.com")

      // Plus tags should be preserved for non-Gmail
      expect(normalizeEmail("user+tag@example.com")).toBe(
        "user+tag@example.com"
      )
      expect(normalizeEmail("test+spam@outlook.com")).toBe(
        "test+spam@outlook.com"
      )

      // Both preserved
      expect(normalizeEmail("jane.doe+home@example.com")).toBe(
        "jane.doe+home@example.com"
      )
    })

    test("handles gmail.com with different cases", () => {
      expect(normalizeEmail("test@GMAIL.COM")).toBe("test@gmail.com")
      expect(normalizeEmail("test@Gmail.Com")).toBe("test@gmail.com")
      expect(normalizeEmail("jane.doe@GMAIL.COM")).toBe("janedoe@gmail.com")
    })
  })

  describe("edge cases", () => {
    test("handles empty plus tag in Gmail", () => {
      expect(normalizeEmail("test+@gmail.com")).toBe("test@gmail.com")
    })

    test("handles multiple plus signs in Gmail (only first is special)", () => {
      expect(normalizeEmail("test+tag+more@gmail.com")).toBe("test@gmail.com")
    })

    test("handles email with only dots in local part", () => {
      expect(normalizeEmail("...@gmail.com")).toBe("@gmail.com")
    })

    test("handles simple email without special characters", () => {
      expect(normalizeEmail("simple@example.com")).toBe("simple@example.com")
      expect(normalizeEmail("test@gmail.com")).toBe("test@gmail.com")
    })
  })
})
