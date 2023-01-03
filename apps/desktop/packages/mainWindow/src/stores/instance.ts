/* eslint-disable no-unused-vars */
import { createStore } from "solid-js/store";

export interface Instance {
  name: string;
  mods: Mod[];
  minecraftVersion: string;
  modloader: Modloader;
  modloaderVersion: string;
  playedTime: number;
}

export enum Modloader {
  Vanilla = "Vanilla",
  Forge = "Forge",
  Fabric = "Fabric",
}

export interface Mod {
  name: string;
  version: string;
}

const [store, setStore] = createStore({} as Instance);

export default store;
