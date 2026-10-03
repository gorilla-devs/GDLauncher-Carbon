type Notice = NonNullable<
  Awaited<ReturnType<typeof window.takeRuntimeMoveNotice>>
>

/**
 * Opens the notice about a finished data folder move. It comes back on every start
 * from the new folder until the user acknowledges it in the modal.
 */
export async function showRuntimeMoveNotice(
  open: (notice: Notice) => void
): Promise<void> {
  try {
    const notice = await window.takeRuntimeMoveNotice?.()
    if (notice) open(notice)
  } catch (error) {
    console.error("Could not read the notice about the data folder move", error)
  }
}

type Unavailable = NonNullable<
  Awaited<ReturnType<typeof window.getUnavailableRuntimePath>>
>

/**
 * Opens the notice that this start could not open the data folder (its drive
 * not connected, say) and uses the default one. Every such start shows it.
 */
export async function showUnavailableRuntimePath(
  open: (value: Unavailable) => void
): Promise<void> {
  try {
    const value = await window.getUnavailableRuntimePath?.()
    if (value) open(value)
  } catch (error) {
    console.error("Could not read whether the data folder is available", error)
  }
}
