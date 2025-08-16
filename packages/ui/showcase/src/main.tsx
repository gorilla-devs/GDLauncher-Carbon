import { render } from "solid-js/web"
import { RouterProvider, createRouter } from "@tanstack/solid-router"
import { routeTree } from "./routeTree.gen"
import "@unocss/reset/tailwind.css"
import "virtual:uno.css"
import "../../src/style.css"

const router = createRouter({ routeTree })

declare module "@tanstack/solid-router" {
  interface Register {
    router: typeof router
  }
}

const root = document.getElementById("root")

if (import.meta.env.DEV && !(root instanceof HTMLElement)) {
  throw new Error(
    "Root element not found. Did you forget to add it to your index.html? Or maybe the id attribute got misspelled?"
  )
}

render(() => <RouterProvider router={router} />, root!)
