export type ThemeErrorKey =
  | "settings:_trn_theme_error_invalid"
  | "settings:_trn_theme_error_too_new"
  | "settings:_trn_theme_error_not_found"
  | "settings:_trn_theme_error_unreadable"

/** The translated sentence for a core `THEME_*` error code. Anything else,
 *  including disk errors that carry no code, gets the generic one. */
export function themeErrorKey(code: string | null | undefined): ThemeErrorKey {
  switch (code) {
    case "THEME_INVALID":
      return "settings:_trn_theme_error_invalid"
    case "THEME_TOO_NEW":
      return "settings:_trn_theme_error_too_new"
    case "THEME_NOT_FOUND":
      return "settings:_trn_theme_error_not_found"
    default:
      return "settings:_trn_theme_error_unreadable"
  }
}

/** The sentence to show for a theme action the core refused with a `THEME_*`
 *  code, which the global error handler only logs. Null for any other error:
 *  the global error toast already reports it. */
export function themeRefusalKey(
  code: string | null | undefined
): ThemeErrorKey | null {
  return code?.startsWith("THEME_") ? themeErrorKey(code) : null
}
