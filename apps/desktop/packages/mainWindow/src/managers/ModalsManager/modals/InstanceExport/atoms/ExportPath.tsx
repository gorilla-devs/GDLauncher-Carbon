import { useTransContext } from "@gd/i18n";
import { Input } from "@gd/ui";
import { createEffect, createSignal } from "solid-js";
import { setPayload, payload } from "..";

const ExportPath = () => {
  const [path, setPath] = createSignal<string | undefined>(undefined);
  const [inputValue, setInputValue] = createSignal(path());
  const [t] = useTransContext();

  return (
    <div class="flex flex-col pt-4 gap-2 w-full">
      <span>{`${t("instance.export_path")} :`}</span>
      <div class="flex gap-2">
        <Input
          value={path()}
          onInput={(e) => {
            setInputValue(e.currentTarget.value);
          }}
          onBlur={() => {
            if (inputValue() && inputValue() !== path()) {
              setPath(inputValue());
              setPayload({ ...payload, save_path: inputValue() });
            }
          }}
          class="flex-1"
          inputColor="bg-darkSlate-900"
          icon={
            <div
              onClick={() => {
                setPath("");
              }}
              class="i-material-symbols:close"
            ></div>
          }
        />
        <div class="flex items-center justify-center p-2 bg-[#1D2028] rounded-lg">
          <div
            onClick={async () => {
              const result = await window.openFileDialog({
                title: "Select Runtime Path",
                defaultPath: path() || "",
                properties: ["openFile", "openDirectory"]
              });

              if (result.canceled) {
                return;
              }

              setPath(result.filePaths[0]);

              setPayload({ ...payload, save_path: result.filePaths[0] });
            }}
            class="i-material-symbols:folder-open-outline text-2xl  cursor-pointer"
          />
        </div>
      </div>
    </div>
  );
};
export default ExportPath;
