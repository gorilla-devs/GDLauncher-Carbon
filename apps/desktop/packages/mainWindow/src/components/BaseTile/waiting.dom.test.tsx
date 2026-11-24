import { afterEach, expect, it, vi } from "vitest"
import { render } from "solid-js/web"
import { BaseTile } from "./index"

vi.mock("@gd/ui", async () => {
  const fixture = await import("@/test/uiFormsRealButton.fixture")
  return { ...fixture, PRESS_CLASSES_LIGHT: "" }
})
vi.mock("@gd/i18n", () => ({
  Trans: (p: any) => <span>{p.key}</span>,
  useTransContext: () => [(k: string) => k]
}))
vi.mock("@gd/i18n/helpers", () => ({ getTaskTranslationKey: (k: string) => k }))

let dispose: (() => void) | undefined
afterEach(() => {
  dispose?.()
  dispose = undefined
  document.body.textContent = ""
})

const mount = (props: {
  isLoading?: boolean
  waitingDetail?: string
  onLeaveWaiting?: () => void
  onClick?: () => void
}) => {
  const root = document.createElement("div")
  document.body.append(root)
  dispose = render(
    () => (
      <BaseTile
        name="Create: Astral"
        size={2}
        isLoading={props.isLoading ?? false}
        isWaiting
        isRunning={false}
        isBusy={false}
        isDeleting={false}
        isMultiSelected={false}
        showCheckbox={false}
        decorationKey="instance:1"
        isDragging={false}
        isDragActive={false}
        canDrag={false}
        onClick={props.onClick}
        shouldSetViewTransition={false}
        viewTransitionPrefix="synthetic"
        playButtonContent="Play"
        infoContent="Info"
        waitingText="In queue"
        waitingDetail={props.waitingDetail}
        onLeaveWaiting={props.onLeaveWaiting}
        leaveWaitingLabel="Leave queue"
      />
    ),
    root
  )
  return {
    detail: () => root.querySelector('[data-testid="tile-waiting-detail"]'),
    leave: () =>
      root.querySelector<HTMLElement>('[data-testid="tile-leave-waiting"]')
  }
}

it("a waiting tile says what it waits for and leaves the queue without opening the instance", () => {
  const leave = vi.fn()
  const open = vi.fn()
  const tile = mount({
    waitingDetail: "after All the Mods 10",
    onLeaveWaiting: leave,
    onClick: open
  })
  expect(document.body.textContent).toContain("In queue")
  expect(tile.detail()?.textContent).toBe("after All the Mods 10")
  expect(tile.leave()?.getAttribute("aria-label")).toBe("Leave queue")
  tile.leave()!.click()
  expect(leave).toHaveBeenCalledTimes(1)
  expect(open).not.toHaveBeenCalled()
  tile
    .leave()!
    .dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true })
    )
  expect(leave).toHaveBeenCalledTimes(2)
})

it("a waiting tile with nothing to say or do shows only that it waits", () => {
  const tile = mount({})
  expect(document.body.textContent).toContain("In queue")
  expect(tile.detail()).toBeNull()
  expect(tile.leave()).toBeNull()
})

it("a tile showing progress has no queue line or way out of it", () => {
  const tile = mount({
    isLoading: true,
    waitingDetail: "after All the Mods 10",
    onLeaveWaiting: () => {}
  })
  expect(tile.detail()).toBeNull()
  expect(tile.leave()).toBeNull()
})
