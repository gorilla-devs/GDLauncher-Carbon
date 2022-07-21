const DEPLOY_ID = import.meta.env.PUBLIC_DEPLOY_ID;
const BRANCH = import.meta.env.PUBLIC_BRANCH;
const COMMIT_REF = import.meta.env.PUBLIC_COMMIT_REF;
export const RELEASE_VERSION = `${BRANCH}@${COMMIT_REF.substring(
  0,
  8
)}-${DEPLOY_ID.substring(0, 8)}`;

export const APP_URLS = {
  cdn: "https://cdn.gdlauncher.com",
};

export const SUPPORTED_LANGUAGES = ["en", "it"];
