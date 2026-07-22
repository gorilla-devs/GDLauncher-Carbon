/// <reference types="vite/client" />
// The "mix" entry augments the electron types this project already has, rather
// than bundling a second copy of them, and is what types `app.overwolf`.
/// <reference types="@overwolf/ow-electron/mix" />

interface ImportMetaEnv {
  NODE_ENV: "development" | "production"
  RUNTIME_PATH: string
  VITE_MAIN_DSN: string
  MODE: string
  DEV: boolean
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

declare const __APP_VERSION__: string
