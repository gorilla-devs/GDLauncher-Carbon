import { rspc } from "@/utils/rspcClient"
import { Checkbox } from "@gd/ui"
import { createEffect, createSignal, For, Match, Show, Switch } from "solid-js"
import { createAsyncEffect } from "@/utils/asyncEffect"
import {
  checkedFiles,
  setCheckedFiles
} from "./ExportCheckboxParent"
import _ from "lodash"
import type { ExploreEntry } from "@gd/core_module/bindings"

interface FileFolder {
  name?: string
  type?: ExploreEntry["type"]
  path?: string[]
}
export function isSubsetOf(needle: string[], haystack: string[]) {
  return needle.every((val, idx) => haystack[idx] === val)
}

const FileCheckbox = (props: { file: FileFolder; name: string }) => {
  const handleChange = (checked: boolean, path: string[]) => {
    if (checked) {
      setCheckedFiles((prev) => [...prev, path])
      return
    }
    setCheckedFiles((prev) =>
      prev.filter((item) => {
        return !isSubsetOf(item, path)
      })
    )
  }

  createEffect(() => {
    const path = [...props.file.path!, props.name]

    const isAlreadyInList = checkedFiles().some((item) => _.isEqual(item, path))

    if (isAlreadyInList) {
      return
    }

    const isAreadyChecked = checkedFiles().some((item) =>
      isSubsetOf(item, path)
    )

    if (isAreadyChecked) {
      setCheckedFiles((prev) => [...prev, path])
    }
  })

  return (
    <Checkbox
      checked={checkedFiles().some((item) =>
        _.isEqual(item, [...props.file.path!, props.name])
      )}
      onChange={(checked: boolean) => {
        handleChange(checked, [...props.file.path!, props.name])
      }}
      children={<span>{props.name}</span>}
    />
  )
}

const ExportCheckbox = (props: {
  folder: FileFolder
  initialData: ExploreEntry[] | undefined
  instanceId: number
}) => {
  const [isOpen, setIsOpen] = createSignal(false)
  const [contents, setContents] = createSignal<ExploreEntry[] | null>(null)
  const rspcContext = rspc.useContext()

  createAsyncEffect((isStale) => {
    const currentIsOpen = isOpen()
    const currentContents = contents()
    const currentPath = props.folder.path

    if (!currentIsOpen && currentContents === null && currentPath) {
      rspcContext.client
        .query([
          "instance.explore",
          {
            instance_id: props.instanceId,
            path: currentPath
          }
        ])
        .then((res) => {
          // Check if state hasn't changed during async operation
          if (!isStale()) {
            setContents(res)
          }
        })
        .catch((error) => {
          console.error("Failed to explore instance folder:", error)
        })
    }
  })

  const handleChange = (checked: boolean, path: string[]) => {
    if (checked) {
      setCheckedFiles((prev) => [...prev, path])
      return
    }
    setCheckedFiles((prev) =>
      prev.filter((item) => !isSubsetOf(item, path) && !isSubsetOf(path, item))
    )
  }

  createEffect(() => {
    const path = _.cloneDeep(props.folder.path!)

    const isAlreadyInList = checkedFiles().some((item) => _.isEqual(item, path))

    if (isAlreadyInList) {
      return
    }

    const isAreadyChecked = checkedFiles().some((item) =>
      isSubsetOf(item, path)
    )

    const currentContents = contents() ?? []
    const isAllChildrenChecked =
      !isAreadyChecked &&
      checkedFiles().filter(
        (item) => item.length - path.length === 1 && isSubsetOf(path, item)
      ).length === currentContents.length &&
      currentContents.length !== 0

    if (isAreadyChecked || isAllChildrenChecked) {
      setCheckedFiles((prev) => [...prev, path])
    }
  })

  return (
    <div class="flex flex-col p-1">
      <Show when={props.folder.name}>
        <div class="flex items-center gap-2">
          <div
            onClick={() => {
              setIsOpen(!isOpen())
            }}
            class={`${
              isOpen()
                ? "i-ep:arrow-down-bold"
                : "i-ep:arrow-down-bold rotate-[270deg]"
            } bg-darkSlate-500`}
          />
          <Checkbox
            indeterminate={checkedFiles().some((item) =>
              isSubsetOf(props.folder.path!, item)
            )}
            checked={checkedFiles().some((item) =>
              _.isEqual(item, props.folder.path!)
            )}
            onChange={(checked: boolean) => {
              handleChange(checked, props.folder.path!)
            }}
            children={<span>{props.folder.name}</span>}
          />
        </div>
      </Show>
      <div style={{ "margin-left": !props.initialData ? "20px" : "" }}>
        <Show when={isOpen() || props.initialData}>
          <For each={props.initialData || contents() || []}>
            {(item) => (
              <div class="flex flex-row items-center justify-between">
                <Switch>
                  <Match when={item.type === "Directory"}>
                    <ExportCheckbox
                      initialData={undefined}
                      instanceId={props.instanceId}
                      folder={{
                        name: item.name,
                        type: item.type,
                        path: [...props.folder.path!, item.name]
                      }}
                    />
                  </Match>
                  <Match when={item.type !== "Directory"}>
                    <div class="flex items-center gap-2 p-1">
                      <div class="h-[16px] w-[16px]" />
                      <FileCheckbox name={item.name} file={props.folder} />
                    </div>
                  </Match>
                </Switch>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  )
}

export default ExportCheckbox
