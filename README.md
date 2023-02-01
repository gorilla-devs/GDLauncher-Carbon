# Carbon

Track https://github.com/cawa-93/vite-electron-builder and https://github.com/electron-vite/electron-vite-boilerplate for electron v20 update.

Check https://github.com/anubhavsrivastava/awesome-ui-component-library#react for UI libs.
https://flowbite.com/

Check https://github.com/vercel/turborepo/tree/main/examples/with-docker for turborepo examples.

## Requirements

- Node 16.16.0
- Rust 1.66.0

## Pnpm

At this point make sure you have pnpm installed:

`npm install -g pnpm`

## Install Dependencies

`pnpm i`

## Development

### Run app in dev mode

First of all you'll need to generate the prisma code. To do that run

`pnpm prisma:generate`

Now you can run the native core in watch mode

`pnpm watch:core`

Now open a new terminal and run the actual app

`pnpm watch:app`

## Production

### Build Native Core

To build native code please run

`cd packages/core_module && pnpm build:[rust_target]`

### Build Entire App

`pnpm build:[target]`

### Generate DB migration

To generate a new migration please run

`pnpm prisma-migrate --name [migration_name]`

Replace `[migration_name]` with the name of the migration you want to create.

## Test

To run tests please run

`pnpm test`

## Lint

To run lint please run

`pnpm lint`

## Release

`pnpm build-{mac|win|linux}-{arm64|x64}`
