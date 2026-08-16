#!/usr/bin/env python3
r"""EXPERIMENTAL binary patch for GDLauncher Carbon 2.0.40 (Modrinth crash fix).

Fixt den RSPC-Start-Crash `unknown variant `minecraft_java_server`` im 2.0.40
AppImage durch Umbiegen des "unknown"-Pointerslots der ProjectType-Variantentabelle
auf einen String `minecraft_java_server` in einem .rodata-Padding-Run.

WICHTIG — bitte vorher lesen:
- NUR stdlib (keine externen Abhängigkeiten).
- Die Enum-Namen liegen im Binary als kontinuierlicher, NICHT-NUL-terminierter
  String-Pool vor ("unknown" wird direkt von "GalleryItem" gefolgt). Das Matching
  erfolgt vermutlich über (Pointer, Länge) statt über NUL-Terminator. Deshalb ist
  dieser Patch HEURISTISCH: Der Pointer wird umgebogen, die String-Länge im Code
  bleibt aber ggf. 7 ("unknown"). Es ist NICHT garantiert, dass die neue Variante
  greift — der `#[serde(other)]`-Catch-all der 2.0.40-Pointertabelle existiert
  nicht, daher kann ein zukünftiger neuer Modrinth-Typ wieder crashen.
- Empfohlene Alternative (chillig): `downgrade-gdlauncher-carbon-2.0.38.sh`
  (2.0.38 enthält den Catch-all bereits).
- Vor Weitergabe: gepatchte App einmal real starten und Kategorien-Seite prüfen.

Nutzerverantwortung: Bei zweifelhafter Byte-Struktur bricht das Script ab
(strict mode). Im Zweifel `--dry-run` zuerst, dann `--apply`.

Verwendung:
    python3 patch-gdlauncher-2.0.40.py GDLauncher__2.0.40__linux__x64.AppImage --dry-run
    python3 patch-gdlauncher-2.0.40.py GDLauncher__2.0.40__linux__x64.AppImage --apply
"""

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile

EXPECTED_SHA256 = "dd7c01f333ce5fcdf3c693e246d2e829bf8204917a6b1ef2dd67d0c845db0336"

# Offsets im extrahierten core_module (Datei-Offsets == Vaddr im PIE-Segment)
POINTER_SLOT_UNKNOWN = 0x2ED10A8   # LE u64 -> Zeiger auf "unknown"-String
STRING_UNKNOWN = 0x2586925         # "unknown" (7 Bytes, NUL-freier Pool)
PADDING_RUN = 0x256B164            # .rodata Zero-Run >= 32 Bytes (verifiziert)
NEEDED = 20                        # "minecraft_java_server" (19) + NUL

NEW_STRING = b"minecraft_java_server\x00"


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_u64(data, off):
    return int.from_bytes(data[off : off + 8], "little")


def verify_appimage(path):
    """Prüft SHA256 gegen die bekannte 2.0.40-Summe."""
    actual = sha256_of(path)
    if actual != EXPECTED_SHA256:
        raise SystemExit(
            f"FEHLER: SHA256-Mismatch.\n  erwartet: {EXPECTED_SHA256}\n  aktuell:  {actual}\n"
            "Abbruch — das ist vermutlich nicht das bekannte 2.0.40-AppImage."
        )
    print(f"SHA256 OK ({EXPECTED_SHA256[:16]}...)")


def extract_appimage(appimage_path):
    """Extrahiert das AppImage nach squashfs-root/ im Zielverzeichnis."""
    appimage_abs = os.path.abspath(appimage_path)
    base = os.path.dirname(appimage_abs)
    outdir = os.path.join(base, "squashfs-root")
    if os.path.exists(outdir):
        shutil.rmtree(outdir)
    subprocess.run(
        [appimage_abs, "--appimage-extract"], cwd=base, check=True, capture_output=True
    )
    core = os.path.join(outdir, "resources", "binaries", "core_module")
    if not os.path.isfile(core):
        raise SystemExit("FEHLER: core_module nicht gefunden nach Extraktion.")
    return core


def verify_core(data, verbose=True):
    """Strikte Byte-Verifikation. Wirft bei Abweichung. Gibt Padding-Start zurück."""
    if len(data) <= STRING_UNKNOWN + 7:
        raise SystemExit("FEHLER: Datei zu klein — Offsets ungültig.")

    ptr = read_u64(data, POINTER_SLOT_UNKNOWN)
    if ptr != STRING_UNKNOWN:
        raise SystemExit(
            f"FEHLER: Pointer-Slot 'unknown' bei 0x{POINTER_SLOT_UNKNOWN:x} = "
            f"0x{ptr:x}, erwartet 0x{STRING_UNKNOWN:x}. Abbruch."
        )
    if verbose:
        print(f"Pointer-Slot 'unknown'  OK: 0x{POINTER_SLOT_UNKNOWN:x} -> 0x{ptr:x}")

    if data[STRING_UNKNOWN : STRING_UNKNOWN + 7] != b"unknown":
        raise SystemExit(
            f"FEHLER: String bei 0x{STRING_UNKNOWN:x} ist nicht 'unknown'. Abbruch."
        )
    if verbose:
        print(
            f"String 'unknown'        OK: 0x{STRING_UNKNOWN:x} = "
            + repr(data[STRING_UNKNOWN : STRING_UNKNOWN + 7])
        )

    # Folge-Byte dokumentieren (NUL-freier Pool -> 'unknownGalleryItem...')
    nxt = data[STRING_UNKNOWN + 7 : STRING_UNKNOWN + 16]
    if verbose:
        print(
            f"Folge-Bytes (Pool)      : {nxt!r}  (kein NUL — Längen-basiertes Matching, heuristisch)"
        )

    # Padding-Run muss >= NEEDED Null-Bytes sein
    run = data[PADDING_RUN : PADDING_RUN + NEEDED]
    if run != b"\x00" * NEEDED:
        raise SystemExit(
            f"FEHLER: Padding-Run bei 0x{PADDING_RUN:x} ist nicht {NEEDED}x00. Abbruch."
        )
    if verbose:
        print(f"Padding-Run             OK: 0x{PADDING_RUN:x} .. +{NEEDED} (Zero-only)")


def patch(data):
    """Schreibt den neuen String in den Padding-Run und biegt den Pointer um."""
    data = bytearray(data)
    new_ptr = PADDING_RUN
    data[new_ptr : new_ptr + len(NEW_STRING)] = NEW_STRING
    data[POINTER_SLOT_UNKNOWN : POINTER_SLOT_UNKNOWN + 8] = (
        new_ptr.to_bytes(8, "little")
    )
    return bytes(data)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("appimage", help="Pfad zum GDLauncher 2.0.40 AppImage")
    ap.add_argument(
        "--dry-run",
        action="store_true",
        help="Nur prüfen und Bericht ausgeben, nichts schreiben.",
    )
    ap.add_argument("--apply", action="store_true", help="Patch tatsächlich anwenden.")
    args = ap.parse_args()

    if args.apply and args.dry_run:
        raise SystemExit("FEHLER: --apply und --dry-run schließen sich aus.")
    if not args.apply:
        args.dry_run = True

    appimage = args.appimage
    if not os.path.isfile(appimage):
        raise SystemExit(f"FEHLER: Datei nicht gefunden: {appimage}")

    print("=== 1. SHA256-Verifikation ===")
    verify_appimage(appimage)

    print("=== 2. Extraktion ===")
    core = extract_appimage(appimage)
    print(f"core_module: {core}")

    with open(core, "rb") as f:
        data = f.read()

    print("=== 3. Byte-Verifikation ===")
    verify_core(data)

    if args.dry_run:
        print("\n=== DRY-RUN — es wurde NICHTS geschrieben. ===")
        print(
            "Patch würde ausführen: 'minecraft_java_server\\x00' nach "
            f"0x{PADDING_RUN:x}, Pointer 0x{POINTER_SLOT_UNKNOWN:x} -> 0x{PADDING_RUN:x}."
        )
        return

    print("=== 4. Patchen ===")
    patched = patch(data)

    base = os.path.dirname(os.path.abspath(appimage))
    outdir = os.path.join(base, "gdl-2.0.40-patched")
    if os.path.exists(outdir):
        shutil.rmtree(outdir)
    shutil.copytree(os.path.join(base, "squashfs-root"), outdir)
    patched_core = os.path.join(outdir, "resources", "binaries", "core_module")
    with open(patched_core, "wb") as f:
        f.write(patched)
    os.chmod(patched_core, 0o755)

    print(f"OK: Gepatchtes Verzeichnis: {outdir}")
    print("Start via: ./gdl-2.0.40-patched/AppRun  (kein Repack nötig)")
    print("Original-AppImage bleibt als Rollback liegen.")
    print(
        "\nHINWEIS (Limits): Fixt nur 'minecraft_java_server'. Das String-Matching im "
        "Pool ist längen-basiert (kein NUL), daher ist der Patch heuristisch — bitte "
        "die App einmal real starten und die Kategorien-Seite prüfen. Neue zukünftige "
        "Modrinth-Typen crashen wieder; dafür 2.0.38 (downgrade-Script) verwenden."
    )


if __name__ == "__main__":
    main()
