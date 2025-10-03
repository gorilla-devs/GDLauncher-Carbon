import { createRootRoute, Outlet } from "@tanstack/solid-router"
import { ThemeProvider } from "../lib/theme-context"
import Layout from "../components/Layout"

export const Route = createRootRoute({
  component: () => (
    <ThemeProvider>
      <Layout>
        <Outlet />
      </Layout>
    </ThemeProvider>
  )
})
