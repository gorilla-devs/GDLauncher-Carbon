/* eslint-disable no-unused-vars */
/// <reference types="vite/client" />

interface ImportMetaEnv {
  MODE: string;
  VITE_POSTHOG_KEY: string;
  VITE_MAIN_DSN: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare module "*.riv" {
  const content: any;
  export default content;
}

declare const __APP_VERSION__: string;
