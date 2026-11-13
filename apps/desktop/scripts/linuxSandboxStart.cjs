const fs = require("fs")
const path = require("path")

/** The name the Electron binary is kept under next to its start script. */
const realBinaryName = (executable) => `${executable}-bin`

/**
 * The script installed under the executable's own name on Linux.
 *
 * Chromium sets up its sandbox before any of the app's code runs and aborts,
 * with no window, when it cannot: user namespaces blocked (Ubuntu 24.04 and
 * later) and no root-owned chrome-sandbox next to the binary, which an
 * AppImage or an extracted ZIP can never have. The script starts the app
 * without the sandbox on exactly those systems and unchanged everywhere else.
 * A deb or rpm install keeps the sandbox: its AppArmor profile is attached to
 * this script's path and inherited by what it starts, or its chrome-sandbox
 * is setuid root.
 */
function startScript(executable) {
  return `#!/bin/bash
here="$(dirname "$(readlink -f "\${BASH_SOURCE[0]}")")"
real="$here/${realBinaryName(executable)}"

sandbox_works() {
  # The setuid helper a deb or rpm install sets up where namespaces are off.
  [ "$(stat -c '%u %a' "$here/chrome-sandbox" 2>/dev/null)" = "0 4755" ] && return 0
  # What Chromium does: a user namespace with its ids mapped.
  if command -v unshare >/dev/null 2>&1; then
    unshare --user --map-root-user true >/dev/null 2>&1
    return
  fi
  [ "$(cat /proc/sys/kernel/apparmor_restrict_unprivileged_userns 2>/dev/null)" != 1 ] &&
    [ "$(cat /proc/sys/kernel/unprivileged_userns_clone 2>/dev/null)" != 0 ] &&
    [ "$(cat /proc/sys/user/max_user_namespaces 2>/dev/null)" != 0 ]
}

for argument in "$@"; do
  [ "$argument" = "--no-sandbox" ] && exec "$real" "$@"
done
if sandbox_works; then
  exec "$real" "$@"
fi
echo "GDLauncher: the Chromium sandbox is not available on this system, starting without it" >&2
exec "$real" --no-sandbox "$@"
`
}

/**
 * Moves the packed Electron binary aside and puts the start script in its
 * place. Running it again on the same folder changes nothing.
 */
function installLinuxSandboxStart(appOutDir, executable) {
  const start = path.join(appOutDir, executable)
  const real = path.join(appOutDir, realBinaryName(executable))
  if (!fs.existsSync(real)) fs.renameSync(start, real)
  fs.writeFileSync(start, startScript(executable), { mode: 0o755 })
  fs.chmodSync(start, 0o755)
}

module.exports = { installLinuxSandboxStart, realBinaryName, startScript }
