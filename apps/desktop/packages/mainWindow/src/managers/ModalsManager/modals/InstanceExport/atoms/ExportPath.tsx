import { useTransContext } from "@gd/i18n"
import { Input } from "@gd/ui"
import { createSignal } from "solid-js"
import { setPayload, payload } from ".."

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
            if (inputValue() && inputValue() !== path()) {
              setPath(inputValue())
              setPayload({ ...payload, save_path: inputValue() })
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
        <div class="flex items-center justify-center rounded-lg bg-[#1D2028] p-2">
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
                        : "MRPACK Files",
                    extensions: [
                      payload.target === "Curseforge" ? "zip" : "mrpack"
                    ]
                  }
                ]
              })

              if (result.canceled) {
                return
              }

              setPath(result.filePath)

              setPayload({ ...payload, save_path: result.filePath })
            }}
          />
        </div>
      </div>
    </div>
  )
}
export default ExportPath
