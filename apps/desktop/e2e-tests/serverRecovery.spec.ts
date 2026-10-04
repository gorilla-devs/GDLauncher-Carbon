/**
 * Server process markers an earlier session left behind never lock a server.
 *
 * `.gdl_server.pid` records the JVM of a running server. A crash, a power
 * loss, a forced reboot or (on Windows) closing 2.0.40 with a server up
 * leaves it behind. The core used to keep it for as long as any process held
 * that PID, show the server as Stopped and refuse Start, Delete, Repair and
 * every edit with "restart the launcher", which did not help.
 *
 * 1. **Reused PID.** A marker whose PID now belongs to a process that is not
 *    Java (here the test runner itself) is removed at startup, as 2.0.40
 *    did, and Start goes ahead: it reaches the EULA dialog of the unprepared
 *    server instead of a refusal.
 * 2. **Left running (Linux).** A marker naming a live JVM (a copy of `sleep`
 *    named `java`, with the exact boot/ticks identity the core records) shows
 *    the server as running from a previous session, says the console is
 *    unavailable, and Stop ends that process and clears the marker, so the
 *    server reads Stopped again without a restart. On Windows the same Stop
 *    is a confirmed Force stop and on macOS the identity is an epoch start
 *    time; the core's unit and integration tests cover both rules.
 * 3. **Ends on its own (Linux).** The same server whose JVM then exits
 *    without the launcher (stopped in-game, a crash, Task Manager) reads
 *    Stopped again within seconds and its marker is gone, with no click.
 *
 * Servers are seeded straight into `gdl_conf.db` between two launches: the
 * first launch logs in and creates the database, the second one finds the
 * server and its marker, as an upgrade would.
 *
 * Own harness per test: each closes and relaunches the app.
 */

import { spawn, type ChildProcess } from "node:child_process"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { DatabaseSync } from "node:sqlite"
import { expect, test } from "@playwright/test"
import type { ElectronApplication, Page } from "playwright"
import {
  attachCoreLogOnFailure,
  closeAppAndCore,
  isCoreModulePresent,
  launchApp,
  type LaunchOptions
} from "./fixtures/electronApp.js"
import { startHarness, stopHarness, type Harness } from "./fixtures/mockIdp.js"
import { completeLogin, dismissStartupModals } from "./fixtures/login.js"
import { byTestId } from "./helpers/selectors.js"
import { isPidAlive } from "./helpers/processes.js"

/** `PID_FILE_NAME` in `managers/server/provider.rs`. */
const SERVER_PID_FILE = ".gdl_server.pid"
const STOP_TIMEOUT = 45_000

/** Anchors on the server page (`pages/Library/Server`). */
const SERVER_IDS = {
  statusBadge: "server-status-badge",
  mainAction: "server-main-action",
  recoveredNotice: "server-recovered-notice",
  noConsole: "server-recovered-no-console"
} as const

interface RunningApp {
  app: ElectronApplication
  page: Page
}

function launchOptions(harness: Harness): LaunchOptions {
  return {
    runtimePath: harness.runtimePath,
    userDataPath: harness.userDataPath,
    runId: harness.runId,
    baseApi: `${harness.mock.url}/gdl`,
    e2eAuthBase: harness.mock.url,
    e2eEntitlementKey: harness.entitlementKeyPath,
    e2eUpdateFeed: `${harness.mock.url}/updates/`
  }
}

/**
 * Logs in on a fresh runtime path, which creates the database, then closes
 * the launcher and waits for its core to exit so the database can be seeded.
 */
async function firstLaunch(
  harness: Harness,
  launchOpts: LaunchOptions
): Promise<void> {
  const { app, page } = await launchApp(launchOpts)
  try {
    await completeLogin(page, harness)
    await dismissStartupModals(page)
  } finally {
    await closeAppAndCore(app, { strict: true })
  }
}

/**
 * Adds a vanilla server to the default group of a closed launcher's
 * database and creates its folder, as the server list reads it on startup.
 * Returns its id and folder. The migration that adds servers
 * (`20260223000000_add_servers`) inserts that "Default" group, so the first
 * launch has created it.
 */
function seedServer(
  runtimePath: string,
  shortpath: string
): { id: number; root: string } {
  const db = new DatabaseSync(path.join(runtimePath, "gdl_conf.db"))
  try {
    const row = db
      .prepare(
        `INSERT INTO "Server" ("name", "shortpath", "index", "groupId", "gameVersion", "serverType")
         VALUES (?, ?, 0, (SELECT "id" FROM "ServerGroup" ORDER BY "groupIndex" LIMIT 1), '1.20.1', 'vanilla')
         RETURNING "id"`
      )
      .get(shortpath, shortpath) as { id: number }
    const root = path.join(runtimePath, "servers", shortpath)
    fs.mkdirSync(path.join(root, "server"), { recursive: true })
    return { id: row.id, root }
  } finally {
    db.close()
  }
}

/**
 * The marker the core writes for `pid` on Linux: epoch start, then the
 * boot id and start ticks that identify the process across clock steps
 * (`orphan_pid::write_process_file`).
 */
function linuxMarker(pid: number): string {
  const stat = fs.readFileSync(`/proc/${pid}/stat`, "utf8")
  // Fields after the command name start at field 3; start ticks is field 22.
  const fields = stat.slice(stat.lastIndexOf(")") + 2).split(" ")
  const startTicks = Number(fields[19])
  const bootId = fs
    .readFileSync("/proc/sys/kernel/random/boot_id", "utf8")
    .trim()
  const btime = Number(
    /^btime (\d+)$/m.exec(fs.readFileSync("/proc/stat", "utf8"))![1]
  )
  // USER_HZ is 100 on every Linux this suite runs on.
  const epoch = btime + Math.floor(startTicks / 100)
  return `${pid}\n${epoch}\nlinux-boot-ticks-v1 ${bootId} ${startTicks}`
}

/** A process the core takes for a server JVM: `sleep` copied as `java`. */
function spawnFakeJvm(): { child: ChildProcess; dir: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "gdl-e2e-fake-jvm-"))
  const sleep = ["/bin/sleep", "/usr/bin/sleep"].find((p) => fs.existsSync(p))
  if (!sleep) throw new Error("no sleep binary to stand in for a server JVM")
  const java = path.join(dir, "java")
  fs.copyFileSync(sleep, java)
  fs.chmodSync(java, 0o755)
  const child = spawn(java, ["600"], { stdio: "ignore" })
  return { child, dir }
}

async function openServerPage(page: Page, id: number): Promise<void> {
  await page.evaluate((serverId) => {
    window.location.hash = `#/library/server/${serverId}`
  }, id)
  await expect(page.locator(byTestId(SERVER_IDS.statusBadge))).toBeVisible({
    timeout: 30_000
  })
}

test.describe("server markers left by an earlier session", () => {
  // eslint-disable-next-line no-empty-pattern
  test("a marker on a reused PID is removed and Start goes ahead", async ({}, testInfo) => {
    expect(isCoreModulePresent()).toBeTruthy()
    const harness = await startHarness()
    const launchOpts = launchOptions(harness)
    let current: RunningApp | null = null

    try {
      await firstLaunch(harness, launchOpts)

      const { id, root } = seedServer(harness.runtimePath, "e2e-reused-pid")
      // Alive and not Java: the test runner. 2.0.40 wrote two lines.
      const marker = path.join(root, SERVER_PID_FILE)
      fs.writeFileSync(marker, `${process.pid}\n1600000000`)

      current = await launchApp(launchOpts)
      await dismissStartupModals(current.page)
      await openServerPage(current.page, id)

      await test.step("the server reads Stopped and the marker is gone", async () => {
        await expect(
          current!.page.locator(byTestId(SERVER_IDS.statusBadge))
        ).toHaveAttribute("data-status", "stopped")
        expect(fs.existsSync(marker)).toBe(false)
        expect(isPidAlive(process.pid)).toBe(true)
      })

      await test.step("Start reaches the EULA dialog instead of a refusal", async () => {
        await current!.page.locator(byTestId(SERVER_IDS.mainAction)).click()
        await expect(
          current!.page.getByText("Minecraft EULA required")
        ).toBeVisible({ timeout: 30_000 })
      })
    } finally {
      if (current) {
        await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
          () => {}
        )
        await current.app.close().catch(() => {})
      }
      await stopHarness(harness)
    }
  })

  // eslint-disable-next-line no-empty-pattern
  test("a server left running that then ends on its own reads Stopped without a click", async ({}, testInfo) => {
    test.skip(
      process.platform !== "linux",
      "seeds the Linux boot/ticks identity of a stand-in JVM"
    )
    expect(isCoreModulePresent()).toBeTruthy()
    const harness = await startHarness()
    const launchOpts = launchOptions(harness)
    let current: RunningApp | null = null
    let jvm: { child: ChildProcess; dir: string } | null = null

    try {
      await firstLaunch(harness, launchOpts)

      const { id, root } = seedServer(harness.runtimePath, "e2e-ends-alone")
      jvm = spawnFakeJvm()
      const pid = jvm.child.pid!
      const marker = path.join(root, SERVER_PID_FILE)
      fs.writeFileSync(marker, linuxMarker(pid))

      current = await launchApp(launchOpts)
      await dismissStartupModals(current.page)
      await openServerPage(current.page, id)
      const page = current.page
      await expect(
        page.locator(byTestId(SERVER_IDS.statusBadge))
      ).toHaveAttribute("data-status", "recoveredRunning")

      await test.step("the JVM exits outside the launcher and the page follows", async () => {
        jvm!.child.kill("SIGKILL")
        await expect
          .poll(() => isPidAlive(pid), { timeout: 10_000 })
          .toBe(false)
        // The watcher checks every 5 s.
        await expect(
          page.locator(byTestId(SERVER_IDS.statusBadge))
        ).toHaveAttribute("data-status", "stopped", { timeout: 20_000 })
        await expect(
          page.locator(byTestId(SERVER_IDS.recoveredNotice))
        ).toHaveCount(0)
        expect(fs.existsSync(marker)).toBe(false)
      })
    } finally {
      if (current) {
        await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
          () => {}
        )
        await current.app.close().catch(() => {})
      }
      if (jvm) {
        if (jvm.child.pid && isPidAlive(jvm.child.pid))
          jvm.child.kill("SIGKILL")
        fs.rmSync(jvm.dir, { recursive: true, force: true })
      }
      await stopHarness(harness)
    }
  })

  // eslint-disable-next-line no-empty-pattern
  test("a server left running shows as running from a previous session and Stop ends it", async ({}, testInfo) => {
    test.skip(
      process.platform !== "linux",
      "seeds the Linux boot/ticks identity of a stand-in JVM"
    )
    expect(isCoreModulePresent()).toBeTruthy()
    const harness = await startHarness()
    const launchOpts = launchOptions(harness)
    let current: RunningApp | null = null
    let jvm: { child: ChildProcess; dir: string } | null = null

    try {
      await firstLaunch(harness, launchOpts)

      const { id, root } = seedServer(harness.runtimePath, "e2e-left-running")
      jvm = spawnFakeJvm()
      const pid = jvm.child.pid!
      const marker = path.join(root, SERVER_PID_FILE)
      fs.writeFileSync(marker, linuxMarker(pid))

      current = await launchApp(launchOpts)
      await dismissStartupModals(current.page)
      await openServerPage(current.page, id)
      const page = current.page

      await test.step("it reads as running from a previous session, without a console", async () => {
        await expect(
          page.locator(byTestId(SERVER_IDS.statusBadge))
        ).toHaveAttribute("data-status", "recoveredRunning")
        await expect(
          page.locator(byTestId(SERVER_IDS.recoveredNotice))
        ).toContainText(String(pid))
        await expect(page.locator(byTestId(SERVER_IDS.noConsole))).toBeVisible()
        expect(isPidAlive(pid), "nothing is stopped before the user asks").toBe(
          true
        )
      })

      await test.step("Stop ends it and the server reads Stopped again", async () => {
        await page.locator(byTestId(SERVER_IDS.mainAction)).click()
        await expect
          .poll(() => isPidAlive(pid), {
            timeout: STOP_TIMEOUT,
            message: "the JVM from the previous session was not stopped"
          })
          .toBe(false)
        await expect(
          page.locator(byTestId(SERVER_IDS.statusBadge))
        ).toHaveAttribute("data-status", "stopped", { timeout: STOP_TIMEOUT })
        expect(fs.existsSync(marker)).toBe(false)
      })
    } finally {
      if (current) {
        await attachCoreLogOnFailure(testInfo, harness.runtimePath).catch(
          () => {}
        )
        await current.app.close().catch(() => {})
      }
      if (jvm) {
        if (jvm.child.pid && isPidAlive(jvm.child.pid))
          jvm.child.kill("SIGKILL")
        fs.rmSync(jvm.dir, { recursive: true, force: true })
      }
      await stopHarness(harness)
    }
  })
})
