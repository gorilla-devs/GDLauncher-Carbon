import { rspc } from "@/utils/rspcClient"
import { committedTheme } from "./active"
import { motion } from "./motion"
import { createSoundBank, createSoundSession } from "./sounds"
import { wireSounds } from "./soundsWiring"

// Wiring only (imports JSX modules, so unit tests never load it): hands the
// app's queries, the saved theme, motion() and the browser to wireSounds,
// which has the logic and the tests.
const initSounds = () => {
  const preferences = rspc.createQuery(() => ({
    queryKey: ["themes.getPreferences"]
  }))
  const session = createSoundSession()

  wireSounds({
    preferences: () => preferences.data,
    sounds: () => committedTheme()?.sounds,
    motion,
    createBank: (sounds, state) =>
      createSoundBank(sounds, state, {
        createAudio: () => new Audio(),
        setTimeout: (cb, ms) => window.setTimeout(cb, ms),
        clearTimeout: (id) => window.clearTimeout(id as number),
        now: () => performance.now(),
        session
      }),
    createTaskList: () =>
      rspc.createQuery(() => ({ queryKey: ["vtask.getTasks"] })),
    now: () => Date.now(),
    document
  })
}

export default initSounds
