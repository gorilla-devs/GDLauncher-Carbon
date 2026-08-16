#!/usr/bin/env bash
set -euo pipefail
# GDLauncher Carbon 2.0.38 enthält bereits den serde(other)-Catch-all für ProjectType.
# 2.0.40 (privater Build) crasht beim Laden der Modrinth-Kategorien.
#
# Hinweis: Ein AUR-Update (`paru -Syu`) springt wieder auf 2.0.40 zurück. Um das zu
# verhindern, in /etc/pacman.conf ergänzen:
#     IgnorePkg = gdlauncher-carbon-bin
#
# Der SHA256 der 2.0.38-Datei ist nicht offiziell verifizierbar (CDN hostet nur
# latest-linux.yml für 2.0.40). Als Kompromiss: AppImage-Magic-Check + Größen-Sanity.
URL="https://cdn-raw.gdl.gg/launcher/GDLauncher__2.0.38__linux__x64.AppImage"
DEST="${1:-/usr/local/bin/gdlauncher-carbon}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Lade 2.0.38 AppImage von $URL ..."
curl -fL "$URL" -o "$TMP/gdl.AppImage"

# Sanity-Check: AppImage-Magic 0x41 0x49 0x02
[ "$(head -c3 "$TMP/gdl.AppImage" | xxd -p)" = "414902" ] || {
    echo "FEHLER: Datei ist kein AppImage (Magic 414902 fehlt). Abbruch." >&2
    exit 1
}

SIZE="$(stat -c %s "$TMP/gdl.AppImage")"
[ "$SIZE" -gt 100000000 ] || {
    echo "FEHLER: Datei verdächtig klein (${SIZE} Bytes). Abbruch." >&2
    exit 1
}

install -Dm755 "$TMP/gdl.AppImage" "$DEST"
echo "OK: Installiert: $DEST (2.0.38, ${SIZE} Bytes)."
echo "Start via AppImage-Runner, ggf. Desktop-File anpassen."
echo "Tipp: IgnorePkg = gdlauncher-carbon-bin in /etc/pacman.conf setzen."
