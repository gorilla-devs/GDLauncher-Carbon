/* eslint-disable no-unused-vars */
/// <reference types="vite/client" />

interface ImportMetaEnv {
  NODE_ENV: "development" | "production";
  readonly VITE_DEV_SERVER_HOST: string;
  readonly VITE_DEV_SERVER_PORT: string;
  VITE_NAPI_ID: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
