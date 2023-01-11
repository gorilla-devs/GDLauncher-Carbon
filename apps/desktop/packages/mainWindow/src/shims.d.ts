/* eslint-disable no-unused-vars */
import type { JSX } from "solid-js";
import type { AttributifyAttributes } from "@unocss/preset-attributify";
// import type { unocssConfig } from "@gd/config";

declare module "solid-js" {
  namespace JSX {
    interface HTMLAttributes<T> extends AttributifyAttributes {}
  }
}
