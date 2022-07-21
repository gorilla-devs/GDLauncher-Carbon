interface ImportMetaEnv {
  // eslint-disable-next-line @typescript-eslint/naming-convention
  readonly PUBLIC_DEPLOY_ID: string;
  // eslint-disable-next-line @typescript-eslint/naming-convention
  readonly PUBLIC_COMMIT_REF: string;
  // eslint-disable-next-line @typescript-eslint/naming-convention
  readonly PUBLIC_BRANCH: string;
}

// eslint-disable-next-line no-shadow
interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare module "*.scss" {
  const content: { [className: string]: string };
  export = content;
}
