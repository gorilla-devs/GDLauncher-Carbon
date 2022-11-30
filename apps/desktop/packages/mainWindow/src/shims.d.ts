/* eslint-disable no-unused-vars */
import type { JSX } from "solid-js";

declare module "solid-js" {
  namespace JSX {
    interface HTMLAttributes<T>
      extends Partial<
        Record<"uno:hover" | "uno:lg" | "uno:md" | "uno:sm" | "uno:xs", string>
      > {}
  }
}
