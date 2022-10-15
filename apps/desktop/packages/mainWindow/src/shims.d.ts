import type { AttributifyNames } from "@unocss/preset-attributify";

type Prefix = "w:";

declare module "solid-js" {
  namespace JSX {
    interface HTMLAttributes<T>
      extends Partial<Record<AttributifyNames<Prefix>, string>> {}
  }
}
