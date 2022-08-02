# Carbon

Track https://github.com/cawa-93/vite-electron-builder and https://github.com/electron-vite/electron-vite-boilerplate for electron v20 update.

Check https://github.com/anubhavsrivastava/awesome-ui-component-library#react for UI libs.
https://flowbite.com/

Check https://github.com/vercel/turborepo/tree/main/examples/with-docker for turborepo examples.

## Development

To run in dev please run

`npm i -g pnpm`

`pnpm i`

`pnpm dev`

## Release

To build the app for release, you need to use `yarn` due to https://github.com/electron-userland/electron-builder/issues/6289 and https://github.com/pnpm/pnpm/issues/4473

`npm i -g yarn`

`yarn`

`yarn build-{mac|win|linux}-{arm64|x64}`