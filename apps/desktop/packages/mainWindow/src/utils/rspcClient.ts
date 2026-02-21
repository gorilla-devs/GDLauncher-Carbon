import {
  QueryClient,
  QueryCache,
  MutationCache,
  createQuery as solidCreateQuery,
  createMutation as solidCreateMutation,
  type CreateQueryResult,
  type CreateMutationResult,
  type SolidQueryOptions,
  type SolidMutationOptions
} from "@tanstack/solid-query"
import type { Procedures } from "@gd/core_module"
import { toast } from "@gd/ui"
import { dispatchBannedEvent } from "./bannedEventBridge"
import {
  dispatchMemoryWarningEvent,
  type InsufficientMemoryData
} from "./memoryWarningBridge"

// ---------------------------------------------------------------------------
// Procedure type utilities
// ---------------------------------------------------------------------------

type QProcs = Procedures["queries"]
type MProcs = Procedures["mutations"]

type QKey = QProcs["key"]
type MKey = MProcs["key"]

type QInput<K extends QKey> = Extract<QProcs, { key: K }>["input"]
type QResult<K extends QKey> = Extract<QProcs, { key: K }>["result"]

type MInput<K extends MKey> = Extract<MProcs, { key: K }>["input"]
type MResult<K extends MKey> = Extract<MProcs, { key: K }>["result"]

// Map `never` input to `undefined` so callers can do .mutate(undefined)
type MInputVar<K extends MKey> = MInput<K> extends never ? undefined : MInput<K>

type QKeyArr<K extends QKey> = QInput<K> extends never ? [K] : [K, QInput<K>]

// Options types for the public API
type RspcQueryOpts<K extends QKey> = Omit<
  SolidQueryOptions<QResult<K>, RSPCError, QResult<K>, QKeyArr<K>>,
  "queryKey"
> & {
  queryKey: QKeyArr<K>
}

type RspcMutationOpts<K extends MKey, TContext = unknown> = Omit<
  SolidMutationOptions<MResult<K>, RSPCError, MInputVar<K>, TContext>,
  "mutationKey" | "mutationFn"
> & {
  mutationKey: [K] | K
  mutationFn?: (input: MInputVar<K>) => Promise<MResult<K>>
}

// ---------------------------------------------------------------------------
// RSPCError — drop-in replacement for @rspc/client's RSPCError
// ---------------------------------------------------------------------------

export class RSPCError extends Error {
  constructor(message: string) {
    super(message)
    this.name = "RSPCError"
  }
}

// ---------------------------------------------------------------------------
// Global error handler (same logic as the old createClient onError)
// ---------------------------------------------------------------------------

function handleGlobalError(error: Error) {
  console.error("RSPC error:", error)

  try {
    const parsed = JSON.parse(error.message) as {
      cause: { display: string; code?: string; data?: unknown }[]
    }

    const errorCause = parsed.cause?.find((c) => c.code)
    const errorCode = errorCause?.code
    const hasCustomCode = !!errorCode

    if (errorCode === "ACCOUNT_BANNED") {
      dispatchBannedEvent()
      return
    }

    if (errorCode === "INSUFFICIENT_MEMORY" && errorCause?.data) {
      dispatchMemoryWarningEvent(errorCause.data as InsufficientMemoryData)
      return
    }

    if (!hasCustomCode && parsed.cause?.[0]?.display) {
      toast.error(parsed.cause[0].display)
    }
  } catch {
    toast.error(error.message)
  }
}

// ---------------------------------------------------------------------------
// QueryClient (same defaults as before, now with global error caches)
// ---------------------------------------------------------------------------

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: handleGlobalError
  }),
  mutationCache: new MutationCache({
    onError: handleGlobalError
  }),
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      networkMode: "always",
      retry: false
    },
    mutations: {
      networkMode: "always"
    }
  }
})

// ---------------------------------------------------------------------------
// HTTP transport
// ---------------------------------------------------------------------------

export let port: number | null = null

async function rspcFetch<T>(
  method: "query" | "mutation",
  key: string,
  input?: unknown,
  opts?: { signal?: AbortSignal }
): Promise<T> {
  const base = `http://127.0.0.1:${port}/rspc/${key}`

  let resp: Response
  if (method === "query") {
    const url =
      input !== undefined
        ? `${base}?input=${encodeURIComponent(JSON.stringify(input))}`
        : base
    resp = await fetch(url, { signal: opts?.signal })
  } else {
    resp = await fetch(base, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: input !== undefined ? JSON.stringify(input) : null,
      signal: opts?.signal
    })
  }

  if (!resp.ok) {
    throw new RSPCError(
      `HTTP ${resp.status}: ${resp.statusText} for ${method} ${key}`
    )
  }

  const body = await resp.json()

  if (body.result?.type === "error") {
    throw new RSPCError(JSON.stringify(body.result.data))
  }

  return body.result.data as T
}

// ---------------------------------------------------------------------------
// rspc object — same API surface as createSolidQueryHooks<Procedures>()
// ---------------------------------------------------------------------------

export const rspc = {
  createQuery<K extends QKey>(
    optsFn: () => RspcQueryOpts<K>
  ): CreateQueryResult<QResult<K>, RSPCError> {
    return solidCreateQuery((() => {
      const opts = optsFn()
      const key = opts.queryKey[0] as string
      const input: unknown =
        opts.queryKey.length > 1 ? opts.queryKey[1] : undefined

      return {
        ...opts,
        queryFn:
          opts.queryFn ??
          ((ctx: { signal: AbortSignal }) =>
            rspcFetch<QResult<K>>("query", key, input, {
              signal: ctx.signal
            }))
      }
    }) as () => SolidQueryOptions<QResult<K>, RSPCError, QResult<K>, QKeyArr<K>> & {
      initialData?: undefined
    }) as CreateQueryResult<QResult<K>, RSPCError>
  },

  createMutation<K extends MKey, TContext = unknown>(
    optsFn: () => RspcMutationOpts<K, TContext>
  ): CreateMutationResult<MResult<K>, RSPCError, MInputVar<K>, TContext> {
    return solidCreateMutation((() => {
      const { mutationKey: rawKey, mutationFn: customFn, ...rest } = optsFn()
      const key: string = Array.isArray(rawKey) ? rawKey[0] : rawKey
      const normalizedKey: readonly unknown[] = Array.isArray(rawKey)
        ? rawKey
        : [rawKey]

      return {
        ...rest,
        mutationKey: normalizedKey,
        mutationFn:
          customFn ??
          ((input: MInputVar<K>) =>
            rspcFetch<MResult<K>>("mutation", key, input))
      } satisfies SolidMutationOptions<
        MResult<K>,
        RSPCError,
        MInputVar<K>,
        TContext
      >
    }) as () => SolidMutationOptions<
      MResult<K>,
      RSPCError,
      MInputVar<K>,
      TContext
    >) as CreateMutationResult<MResult<K>, RSPCError, MInputVar<K>, TContext>
  },

  useContext() {
    return {
      client: {
        query: <K extends QKey>(
          args: QInput<K> extends never ? [K] : [K, QInput<K>],
          opts?: { signal?: AbortSignal }
        ): Promise<QResult<K>> =>
          rspcFetch<QResult<K>>(
            "query",
            args[0] as string,
            args.length > 1 ? args[1] : undefined,
            opts
          ),
        mutation: <K extends MKey>(
          args: MInput<K> extends never ? [K] : [K, MInput<K>]
        ): Promise<MResult<K>> =>
          rspcFetch<MResult<K>>(
            "mutation",
            args[0] as string,
            args.length > 1 ? args[1] : undefined
          )
      },
      queryClient
    }
  }
}

// ---------------------------------------------------------------------------
// Initialisation (called once when the backend port is known)
// ---------------------------------------------------------------------------

export default function initRspc(_port: number) {
  port = _port

  const createInvalidateQuery = () => {
    let socket: WebSocket

    interface InvalidateOperation {
      key: string
      args: never
    }

    function connect() {
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
        queryClient
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
    createInvalidateQuery
  }
}
