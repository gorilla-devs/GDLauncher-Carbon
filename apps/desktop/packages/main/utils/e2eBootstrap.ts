import fs from "node:fs"
import path from "node:path"
import { randomUUID } from "node:crypto"
import { observeMainProcess, type MainProcessIdentity } from "./e2eMainIdentity"

export const E2E_MAIN_MARKER = "e2e-main-start.json"
export interface E2EMainStart {
  version: 1
  role: "main"
  runId: string
  generation: string
  runtimePath: string
  userDataPath: string
  process: MainProcessIdentity
}

/** Called by the actual main bootstrap before default/snapshot path selection
 * and before electron-log initialization. Released builds use their lazy default.
 */
export function initializeMainUserData(options: {
  e2eBuild: boolean
  e2eUserData?: string
  defaultUserData: () => string
  setPath: (path: string) => void
  setLogPath: (resolve: (fileName: string) => string) => void
  initializeLog: () => void
}): { userDataPath: string; existed: boolean } {
  let userDataPath: string
  if (options.e2eBuild && options.e2eUserData) {
    if (!path.isAbsolute(options.e2eUserData))
      throw new Error("E2E userData must be absolute")
    userDataPath = options.e2eUserData
  } else {
    userDataPath = options.defaultUserData()
  }
  const existed = fs.existsSync(userDataPath)
  options.setPath(userDataPath)
  options.setLogPath((fileName) => path.join(userDataPath, fileName))
  options.initializeLog()
  return { userDataPath, existed }
}

/** E2E-only evidence; it does not participate in production runtime admission.
 * A fresh generation is written on each main startup, including app.relaunch.
 */
export function publishE2EMainStart(options: {
  e2eBuild: boolean
  runId?: string
  userDataPath: string
  runtimePath: string | null
  observe?: () => MainProcessIdentity | null
}): E2EMainStart | null {
  if (!options.e2eBuild || !options.runId) return null
  if (
    !options.runtimePath ||
    !path.isAbsolute(options.runtimePath) ||
    !path.isAbsolute(options.userDataPath)
  )
    throw new Error("Missing owned E2E startup paths")
  const identity = (
    options.observe ?? (() => observeMainProcess(process.pid))
  )()
  if (identity?.role !== "main")
    throw new Error("Cannot identify E2E main startup")
  const marker: E2EMainStart = {
    version: 1,
    role: "main",
    runId: options.runId,
    generation: randomUUID(),
    runtimePath: options.runtimePath,
    userDataPath: options.userDataPath,
    process: identity
  }
  fs.mkdirSync(options.userDataPath, { recursive: true })
  const target = path.join(options.userDataPath, E2E_MAIN_MARKER)
  const staging = `${target}.${marker.generation}.tmp`
  try {
    const fd = fs.openSync(staging, "wx", 0o600)
    try {
      fs.writeFileSync(fd, JSON.stringify(marker))
      fs.fsyncSync(fd)
    } finally {
      fs.closeSync(fd)
    }
    fs.renameSync(staging, target)
  } finally {
    fs.rmSync(staging, { force: true })
  }
  return marker
}

export function readE2EMainStart(userDataPath: string): E2EMainStart | null {
  try {
    const value = JSON.parse(
      fs.readFileSync(path.join(userDataPath, E2E_MAIN_MARKER), "utf8")
    )
    if (
      value.version !== 1 ||
      value.role !== "main" ||
      typeof value.runId !== "string" ||
      !value.runId ||
      typeof value.generation !== "string" ||
      !/^[\da-f-]{36}$/i.test(value.generation) ||
      typeof value.runtimePath !== "string" ||
      !path.isAbsolute(value.runtimePath) ||
      value.userDataPath !== userDataPath ||
      value.process?.role !== "main" ||
      !Number.isSafeInteger(value.process.pid) ||
      value.process.pid <= 0 ||
      typeof value.process.executable !== "string" ||
      !value.process.executable ||
      typeof value.process.startIdentity !== "string" ||
      !value.process.startIdentity
    )
      return null
    return value as E2EMainStart
  } catch {
    return null
  }
}
