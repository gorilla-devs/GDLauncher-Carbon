import { useTransContext } from "@gd/i18n"
import { Input } from "@gd/ui"
import { createSignal } from "solid-js"
import { setPayload, payload } from ".."

const getExpectedExtension = (target: string) =>
  target === "Curseforge" ? ".zip" : target === "Gdlauncher" ? ".gdlpack" : ".mrpack"

const ensureExtension = (filePath: string, target: string) => {
  const ext = getExpectedExtension(target)
  return filePath.endsWith(ext) ? filePath : filePath + ext
}

const ExportPath = () => {
  const [path, setPath] = createSignal<string | undefined>(undefined)
  const [inputValue, setInputValue] = createSignal(path())
  const [t] = useTransContext()

  return (
    <div class="flex w-full flex-col gap-2 pt-4">
      <span>{`${t("instances:_trn_export_path")} :`}</span>
      <div class="flex gap-2">
        <Input
          value={path()}
          onInput={(e) => {
            setInputValue(e.currentTarget.value)
          }}
          onBlur={() => {
            const val = inputValue()
            if (val && val !== path()) {
              const fixed = ensureExtension(val, payload.target)
              setPath(fixed)
              setInputValue(fixed)
              setPayload({ ...payload, save_path: fixed })
            }
          }}
          class="flex-1"
          inputColor="bg-darkSlate-900"
          icon={
            <div
              class="i-material-symbols:close"
              onClick={() => {
                setPath("")
              }}
            />
          }
        />
        <div class="flex items-center justify-center rounded-lg bg-darkSlate-900 p-2">
          <div
            class="i-material-symbols:folder-open-outline cursor-pointer text-2xl"
            onClick={async () => {
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
                    extensions: [
                      payload.target === "Curseforge"
                        ? "zip"
                        : payload.target === "Gdlauncher"
                          ? "gdlpack"
                          : "mrpack"
                    ]
                  }
                ]
              })

              if (result.canceled) {
                return
              }

              const filePath = ensureExtension(result.filePath!, payload.target)
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
