import { recoveryUpdateCheck } from "./recoveryUpdate"

/**
 * The recovery screens' real update check on stand-in elements, for tests
 * that drive it from main's side without a DOM.
 */
export function recoveryUpdateScreen(
  invoke: (name: string, ...args: any[]) => Promise<any>
) {
  let click: () => Promise<void> = async () => {}
  let pushed: (event: unknown, state: any) => void = () => {}
  const button = {
    textContent: "",
    disabled: false,
    style: {} as Record<string, string>,
    addEventListener: (_: string, listener: () => Promise<void>) => {
      click = listener
    }
  }
  const status = {
    textContent: "",
    innerHTML: "",
    style: {} as Record<string, string>
  }
  const ready = recoveryUpdateCheck({
    ipc: {
      invoke,
      on: (_: string, listener: (event: unknown, state: any) => void) => {
        pushed = listener
      }
    } as any,
    button: button as any,
    status: status as any,
    progress: { style: {} } as any,
    bar: { style: {} } as any,
    idle: "",
    noUpdate: () => {},
    oneLine: true
  })
  return {
    button,
    status,
    /** Settles once the screen has read main's state, and checked if idle. */
    ready,
    click: () => click(),
    receive: (state: any) => pushed({}, state)
  }
}
