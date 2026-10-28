import type { Page, Request, Response } from "playwright"

export function rpcRoute(url: string): string | undefined {
  try {
    return /^\/rspc\/([^/]+)$/.exec(new URL(url).pathname)?.[1]
  } catch {
    return undefined
  }
}
export function mutationInput(request: Request): unknown {
  try {
    return request.postDataJSON()
  } catch {
    return undefined
  }
}
export interface RpcResult {
  type: string
  data?: unknown
}

/** Prearmed request-object correlation: this transport has no wire request ID. */
export function observeOperations(page: Page) {
  const requests: Request[] = []
  const results = new Map<Request, RpcResult>()
  const failures = new Map<Request, unknown>()
  const onRequest = (request: Request) => {
    if (request.method() === "POST" && rpcRoute(request.url()))
      requests.push(request)
  }
  const onResponse = (response: Response) => {
    const request = response.request()
    if (!requests.includes(request)) return
    void response
      .json()
      .then((body: { result?: RpcResult }) => {
        if (!body.result || typeof body.result.type !== "string")
          throw new Error("Malformed rspc result")
        results.set(request, body.result)
      })
      .catch((error: unknown) => {
        failures.set(request, error)
      })
  }
  page.on("request", onRequest)
  page.on("response", onResponse)
  return {
    async find(route: string, matches: (input: unknown) => boolean) {
      const matching = requests.filter(
        (r) => rpcRoute(r.url()) === route && matches(mutationInput(r))
      )
      if (matching.length > 1) throw new Error(`Ambiguous ${route} operation`)
      const request = matching[0]
      if (request && failures.has(request)) throw failures.get(request)
      return request && results.has(request)
        ? { request, result: results.get(request)! }
        : undefined
    },
    dispose() {
      page.off("request", onRequest)
      page.off("response", onResponse)
    }
  }
}

export interface InstallOperation {
  wait(startTimeout: number, installTimeout: number): Promise<void>
  dispose(): void
}
export type InstallTarget =
  | { create: (input: unknown) => boolean }
  | {
      route:
        | "instance.prepareInstance"
        | "instance.changeModpack"
        | "instance.repairModpack"
      instance: number
    }

export function armInstallOperation(
  page: Page,
  target: InstallTarget,
  readOutcome: (
    request: Request,
    task: number
  ) => Promise<{ type: string }> = async (request, task) => {
    const url = new URL(request.url())
    url.pathname = "/rspc/vtask.getTaskOutcome"
    url.search = `?input=${encodeURIComponent(JSON.stringify(task))}`
    // Same owned core/auth transport as the accepted request, never a provider.
    const token = (await request.allHeaders())["x-api-token"]
    const response = await page.request.get(url.toString(), {
      headers: token ? { "x-api-token": token } : {}
    })
    const body = await response.json()
    if (body.result?.type !== "response")
      throw new Error("Task outcome query failed")
    return body.result.data
  }
): InstallOperation {
  const observer = observeOperations(page)
  return {
    dispose: observer.dispose,
    async wait(startTimeout, installTimeout) {
      let instance = "instance" in target ? target.instance : undefined
      const started = Date.now()
      let accepted: { request: Request; result: RpcResult } | undefined
      try {
        while (Date.now() - started < startTimeout) {
          if ("create" in target && instance === undefined) {
            const created = await observer.find(
              "instance.createInstance",
              target.create
            )
            if (created) {
              if (
                created.result.type !== "response" ||
                !Number.isInteger(created.result.data)
              )
                throw new Error("Instance creation was not accepted")
              instance = created.result.data as number
            }
          }
          if (instance !== undefined) {
            const route =
              "create" in target ? "instance.prepareInstance" : target.route
            accepted = await observer.find(route, (input) =>
              route === "instance.prepareInstance"
                ? input === instance
                : typeof input === "object" &&
                  input !== null &&
                  "instance" in input &&
                  input.instance === instance
            )
            if (accepted) break
          }
          await page.waitForTimeout(50)
        }
        if (!accepted)
          throw new Error("Current install operation was never accepted")
        if (
          accepted.result.type !== "response" ||
          !Number.isInteger(accepted.result.data)
        )
          throw new Error("Current install operation was refused")
        const task = accepted.result.data as number
        const deadline = Date.now() + installTimeout
        while (Date.now() < deadline) {
          const outcome = await readOutcome(accepted.request, task)
          if (outcome.type === "Succeeded") return
          if (outcome.type !== "Running")
            throw new Error(
              `Install task ${task} did not succeed: ${outcome.type}`
            )
          await page.waitForTimeout(100)
        }
        throw new Error(`Install task ${task} did not finish`)
      } finally {
        observer.dispose()
      }
    }
  }
}
