# Carbon

Track https://github.com/cawa-93/vite-electron-builder and https://github.com/electron-vite/electron-vite-boilerplate for electron v20 update.

Check https://github.com/anubhavsrivastava/awesome-ui-component-library#react for UI libs.
https://flowbite.com/

Check https://github.com/vercel/turborepo/tree/main/examples/with-docker for turborepo examples.

## Requirements

- Node 16.16.0
- Rust 1.66.0

## Env

First of all you need to create a .env file in the root folder of the source, and add the following variable:

`VITE_NAPI_ID=GDL`

## Pnpm

At this point make sure you have pnpm installed:

`npm install -g pnpm`

## Install Dependencies

`pnpm i`

## Development

To run in dev please run

`pnpm dev-app`

## Test

To run tests please run

`pnpm test`

## Lint

To run lint please run

`pnpm lint`

## Release

`pnpm build-{mac|win|linux}-{arm64|x64}`
