import type { AttributifyNames } from "@unocss/preset-attributify";

declare module "solid-js" {
  namespace JSX {
    interface HTMLAttributes<T>
      extends Partial<
        Record<"uno:hover" | "uno:lg" | "uno:md" | "uno:sm" | "uno:xs", string>
      > {}
  }
}
