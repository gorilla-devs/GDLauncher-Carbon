import { RuntimeLoader } from "@rive-app/canvas"
import engine from "@rive-app/canvas/rive.wasm?url"

// The animation engine is shipped with the app. Left alone, the library
// fetches it from a CDN each time, and shows nothing where that CDN cannot be
// reached.
RuntimeLoader.setWasmUrl(engine)
