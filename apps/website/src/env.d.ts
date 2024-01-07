/// <reference path="../.astro/types.d.ts" />
/// <reference types="astro/client" />

namespace App {
  interface Locals {
    // This will allow us to set the cache duration for each page.
    cache(seconds: number): void;
  }
}
