import { useTransContext } from "@gd/i18n"
import { Input } from "@gd/ui"
import { createEffect, createSignal } from "solid-js"
import { setPayload, payload } from ".."

import { archiveSuffix, normalizeArchivePath } from "./archivePath"

const ExportPath = () => {
  const [path, setPath] = createSignal<string | undefined>(payload.save_path)
  const [inputValue, setInputValue] = createSignal(path())
  const [t] = useTransContext()
  createEffect(() => {
    setPath(payload.save_path)
    setInputValue(payload.save_path)
  })

  return (
    <div class="flex w-full flex-col gap-2 pt-4">
      <span>{`${t("instances:_trn_export_path")} :`}</span>
      <div class="flex gap-2">
        <Input
          value={path()}
          onInput={(e) => {
            setInputValue(e.currentTarget.value)
            setPayload("save_path", e.currentTarget.value)
          }}
          onBlur={() => {
            const val = inputValue()
            if (val !== undefined) {
              const fixed = normalizeArchivePath(val, payload.target)
              setPayload("save_path", fixed)
            }
          }}
          class="flex-1"
          inputColor="bg-darkSlate-900"
          icon={
            <div
              class="i-material-symbols:close"
              onClick={() => {
                setPayload("save_path", "")
              }}
            />
          }
        />
        <div class="flex items-center justify-center rounded-lg bg-darkSlate-900 p-2">
          <div
            class="i-material-symbols:folder-open-outline cursor-pointer text-2xl"
            onClick={async () => {
              const suffix = archiveSuffix(payload.target)
              if (!suffix) return
              const result = await window.showSaveDialog({
                title: "Select Runtime Path",
                defaultPath: path() || "",
                filters: [
                  {
                    name:
                      payload.target === "Curseforge"
                        ? "ZIP Files"
                        : payload.target === "Gdlauncher"
                          ? "GDLPack Files"
                          : "MRPACK Files",
                    extensions: [suffix.slice(1)]
                  }
                ]
              })

              if (result.canceled) {
                return
              }

              const filePath = normalizeArchivePath(
                result.filePath,
                payload.target
              )
              setPath(filePath)

              setPayload({ ...payload, save_path: filePath })
            }}
          />
        </div>
      </div>
    </div>
  )
}
export default ExportPath
