/// <reference types="vite/client" />

interface ImportMetaEnv {
  MODE: string;
  VITE_NAPI_ID: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
