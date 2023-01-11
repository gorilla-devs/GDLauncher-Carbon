/* eslint-disable no-unused-vars */
import type { JSX } from "solid-js";
import type { AttributifyNames } from "@unocss/preset-attributify";

type Prefix = "u:";

declare module "solid-js" {
  namespace JSX {
    interface HTMLAttributes<T>
      extends Partial<Record<AttributifyNames<Prefix>, string>> {}
  }
}
