# GDLauncher (Carbon)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](http://makeapullrequest.com)
![Discord](https://img.shields.io/discord/398091532881756161)

## 🎮 What is GDLauncher

GDLauncher is a custom Minecraft launcher written from the ground up in rust and solidJS. Its main goal is to make it easy and enjoyable to manage different Minecraft versions, install modloaders, mods and modpacks from different platforms, bringing the playing and modding experience to the next level!

## What happened to the old GDLauncher?
The old GDLauncher has been rewritten and officially discontinued. You can still find the old code in the [legacy branch](#-history).
You can read more [here](https://gdlauncher.com/en/blog/curseforge-partnership-announcement/)

## Table of Content
<details>
 <summary><strong>Table of Contents</strong> (click to expand)</summary>

- [GDLauncher (Carbon)](#gdlauncher-carbon)
  - [🎮 What is GDLauncher](#-what-is-gdlauncher)
  - [What happened to the old GDLauncher?](#what-happened-to-the-old-gdlauncher)
  - [Table of Content](#table-of-content)
  - [📥 Download](#-download)
  - [🎉 Join our community](#-join-our-community)
  - [🎁 Features](#-features)
  - [▶️ Development](#️-development)
    - [A quick note](#a-quick-note)
    - [Contributing](#contributing)
    - [Requirements](#requirements)
    - [Pnpm](#pnpm)
    - [Install Dependencies](#install-dependencies)
    - [Run app in dev mode](#run-app-in-dev-mode)
    - [Generate DB migration](#generate-db-migration)
  - [🔍 Test](#-test)
  - [\</\> Lint](#-lint)
  - [\</\> Code Formatting](#-code-formatting)
  - [🚚 Production](#-production)
  - [🎓 License](#-license)
  - [™️ Trademark](#️-trademark)
  - [📜 History](#-history)


</details>

<p align="center">
    <img width="600" height="auto" src="https://cdn.gdl.gg/github-readme/home.png" alt="GDLauncher" />
    <img width="300" height="auto" src="https://cdn.gdl.gg/github-readme/modpacks.png" alt="GDLauncher" />
    <img width="300" height="auto" src="https://cdn.gdl.gg/github-readme/mods.png" alt="GDLauncher" />
</p>

## 📥 Download
GDLauncher is currently in alpha and is only available for download through our discord server. You can join our discord server by clicking the button below.

## 🎉 Join our community
Join our official GDLauncher discord server. There you'll find our community and friends to play with along with support for any issues you may have.

<a href='https://discord.gdlauncher.com' target='_blank'><img height='40' style='border:0px;height:50px;' src='https://cdn.gdl.gg/github-readme/Discord-Logo-Wordmark-Color.png' border='0' alt='GDLauncher Discord Server' /></a>

## 🎁 Features
- 🎮 Easily install any minecraft version and **modloader**, including `forge`, `fabric`, `quilt`, `neoforge`
- 📦 Install `addons` from **CurseForge**, **Modrinth** and more!
- 📦 Install `modpacks` from **CurseForge**, **Modrinth** and more!
- ☕ Automatic **Java Manager**. You don't need to have java installed on your system, the launcher will take care of installing the correct java version for you!
- 🔄 Built-in **auto updater**
- 👥 **Multi account** support
- 🐢 Still playing on your grandma's pc from the 80s? Don't worry, we got you covered with **Potato PC Mode**!
- ⬇️ Import/export instances from/to other launchers like **Curseforge**, **MultiMC**, **ATLauncher**, **Technic**, **Prism**, **Modrinth**, **FTB** and more! (WIP)


<!-- Track https://github.com/cawa-93/vite-electron-builder and https://github.com/electron-vite/electron-vite-boilerplate for electron v20 update.

Check https://github.com/anubhavsrivastava/awesome-ui-component-library#react for UI libs.
https://flowbite.com/

Check https://github.com/vercel/turborepo/tree/main/examples/with-docker for turborepo examples. -->

## ▶️ Development

### A quick note
To be able to develop on GDLauncher Carbon locally, you will need to request an API key to `api-keys@gdlauncher.com`. Please include your github username and a short description of what you are planning to do with the API key.

### Contributing
To contribute, please see the [contributing](CONTRIBUTING.md) guide.

### Requirements
- Node 18.x.x
- Rust >= 1.73.0

### Pnpm
At this point make sure you have pnpm installed:

`npm install -g pnpm`

### Install Dependencies

`pnpm i`

### Run app in dev mode
First of all you'll need to generate the prisma code and rspc bindings. To do that run

`pnpm codegen`

Now you can run the native core in watch mode

`pnpm watch:core`
Note: Core module hot reload doesn't currently work on windows

Now open a new terminal and run the actual app

`pnpm watch:app`

### Generate DB migration
To generate a new migration please run

`pnpm prisma:migrate --name {migration_name}`

Replace `{migration_name}` with the name of the migration you want to create.

## 🔍 Test
To run tests please run

`pnpm test`

## </> Lint
To run lint please run

`pnpm lint`

## </> Code Formatting
A [`.editorconfig`](https://editorconfig.org/) is in the repo to normalize inconsistencies your editor may make when saving a file such as indentation and line endings. Make sure the plugin is installed for your editor.


## 🚚 Production
`pnpm build:{win|mac|linux}-{x64|arm64}`


## 🎓 License

GDLauncher and its logo are copyright © 2023 GorillaDevs Inc. All rights reserved.

The software in this repository is released under the Business Source License 1.1 (BSL 1.1) - see the [LICENSE](LICENSE) file for details

## ™️ Trademark
The GDLauncher name and logo are trademarks of GorillaDevs Inc. and may not be used without the express written permission of GorillaDevs Inc.

## 📜 History

- 2014: Davide started learning programming and experimenting with writing a C# launcher
- 2015: Davide rewrote the launcher (still in C#) but with some better designs and features
- 2016: Yet another C# rewrite
- 2018: First Electron rewrite, Davide got into web development and started learning React
- 2019: GDL takes on a new and modern design features Curseforge integrations, massive UI changes, and one-of-a-kind features.
- 2022: GDLauncher Carbon Rewrite [See announcement](https://gdlauncher.com/en/blog/curseforge-partnership-announcement/)

<br>

<img width="600" height="auto" src="https://cdn.gdl.gg/github-readme/launcher_evolution.png" alt="GDLauncher" />


Here you can find the previous versions of the code:

- [Original C# Code](https://github.com/gorilla-devs/GDLauncher_LEGACY-Full-History/tree/csharp_legacy_launcher)
- [First Electron/React Version](https://github.com/gorilla-devs/GDLauncher_LEGACY-Full-History/tree/GDLauncher_old)
- GDLauncher (Discontinued) [Legacy Branch](https://www.github.com/gorilla-devs/GDLauncher)
