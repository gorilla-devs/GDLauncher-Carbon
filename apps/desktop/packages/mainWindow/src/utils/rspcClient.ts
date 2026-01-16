import { QueryClient } from "@tanstack/solid-query"
import { WebsocketTransport, createClient } from "@rspc/client"
import { createSolidQueryHooks } from "@rspc/solid"
import type { Procedures } from "@gd/core_module"
import { toast } from "@gd/ui"

export const rspc = createSolidQueryHooks<Procedures>()
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false, // default: true
      networkMode: "always",
      retry: false
    },
    mutations: {
      networkMode: "always"
    }
  }
})

export let port: number | null = null

export default function initRspc(_port: number) {
  port = _port

  const transport = new WebsocketTransport(`ws://127.0.0.1:${_port}/rspc/ws`)

  const client = createClient<Procedures>({
    transport,
    onError: (error) => {
      console.error("RSPC error:", error)

      // Show toast for errors that don't have custom error codes.
      // Errors with codes (like QUOTA_EXCEEDED, MAX_DOWNLOADS_EXCEEDED) are
      // handled by components with specific translated messages.
      try {
        const parsed = JSON.parse(error.message) as {
          cause: { display: string; code?: string }[]
        }

        // Check if any cause segment has a custom error code
        const hasCustomCode = parsed.cause?.some((c) => c.code)

        // Only show global toast for errors without custom codes
        if (!hasCustomCode && parsed.cause?.[0]?.display) {
          toast.error(parsed.cause[0].display)
        }
      } catch {
        toast.error(error.message)
      }
    }
  })

  const createInvalidateQuery = () => {
    const context = rspc.useContext()
    let socket: WebSocket

    interface InvalidateOperation {
      key: string
      args: never
    }

    function connect() {
      // Create a new WebSocket connection
      socket = new WebSocket(`ws://127.0.0.1:${_port}/invalidations`)

      socket.addEventListener("open", () => {
        console.log("Invalidations channel connected")
      })

      socket.addEventListener("message", (event) => {
        const data = JSON.parse(event.data as never) as InvalidateOperation
        const key = [data.key]
        if (data.args !== null) {
          key.push(data.args)
        }
        context.queryClient
          .invalidateQueries({
            queryKey: key
          })
          .catch((error) => {
            console.error("Invalidations channel error:", error)
          })
      })

      socket.addEventListener("close", () => {
        console.log(
          "Invalidations channel disconnected. Attempting to reconnect..."
        )
        setTimeout(connect, 1000)
      })

      socket.addEventListener("error", (error) => {
        console.error("Invalidations channel error:", error)
        socket.close()
      })
    }

    connect()
  }

  return {
    client,
    createInvalidateQuery
  }
}
