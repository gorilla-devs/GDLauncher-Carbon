/* eslint-disable @typescript-eslint/naming-convention */
// eslint-disable-next-line no-restricted-syntax
import { t as i18nextT } from "i18next";
import type { TFunctionKeys, StringMap, TOptions } from "i18next";

// Simplified version of TFunction from i18next that always returns a string
export interface TFunction {
  // basic usage
  <TKeys extends TFunctionKeys = string>(key: TKeys | TKeys[]): string;
  <
    TKeys extends TFunctionKeys = string,
    TInterpolationMap extends object = StringMap
  >(
    key: TKeys | TKeys[],
    options?: TOptions<TInterpolationMap> | string
  ): string;
  // overloaded usage
  <
    TKeys extends TFunctionKeys = string,
    TInterpolationMap extends object = StringMap
  >(
    key: TKeys | TKeys[],
    defaultValue?: string,
    options?: TOptions<TInterpolationMap> | string
  ): string;
}

export const t: TFunction = (...args) =>
  // eslint-disable-next-line prefer-spread
  i18nextT.apply(null, args) as string;
