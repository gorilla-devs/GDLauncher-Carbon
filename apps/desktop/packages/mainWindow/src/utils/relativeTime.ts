import type { TypedTFunction } from "@gd/i18n"

export function formatRelativeTime(
  t: TypedTFunction,
  dateString: string | null | undefined
): string {
  if (!dateString) return t("instances:_trn_never_played")
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return t("instances:_trn_just_now")
  if (diffMins < 60) return t("instances:_trn_minutes_ago", { count: diffMins })
  if (diffHours < 24) return t("instances:_trn_hours_ago", { count: diffHours })
  if (diffDays < 30) return t("instances:_trn_days_ago", { count: diffDays })
  return date.toLocaleDateString()
}
