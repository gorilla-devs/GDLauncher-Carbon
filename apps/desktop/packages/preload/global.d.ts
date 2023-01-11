/* eslint-disable no-unused-vars */
/// <reference types="vite/client" />

interface ImportMetaEnv {
  MODE: string;
  VITE_POSTHOG_KEY: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
