export function clampMenuX(
  x: number,
  menuWidth: number,
  contentRight: number
): number {
  return Math.max(0, Math.min(x, contentRight - menuWidth))
}
