import { useTransContext } from "@gd/i18n"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from "@gd/ui"
import type { FEJavaEntry } from "@gd/core_module/bindings"
import { Show, createMemo } from "solid-js"
import { sourceLine } from "@/pages/Settings/java/javaOverviewView"
import { labelHints, middleTruncate } from "@/utils/java/javaText"
import { useFormatSize } from "@/utils/java/useFormatSize"

// What a 20rem list fits in text-xs after the source, so the path is never
// cut twice.
const PATH_MAX = 32

/** Picks one of the listed Javas. The value reads "Java 17 · Azul Zulu
 *  17.0.18"; each option adds where the Java comes from, as Settings › Java
 *  lists it. Javas that don't start or are about to be removed can't be
 *  picked. */
const JavaSelect = (props: {
  javas: FEJavaEntry[]
  value: string | null
  onChange: (javaId: string) => void
  /** Names what the Java is for, as the select has no visible label. */
  label?: string
  disabled?: boolean
  class?: string
}) => {
  const [t] = useTransContext()
  const size = useFormatSize()
  const java = (id: string | null | undefined) =>
    props.javas.find((e) => e.id === id)
  const hints = createMemo(() => labelHints(props.javas))

  /** "Java 17 · Azul Zulu 17.0.18", and what tells it from a twin. */
  const Name = (nameProps: { java: FEJavaEntry }) => (
    <span class="flex min-w-0 items-baseline gap-2">
      <span class="text-lightSlate-50 min-w-0 truncate">
        {t("java:_trn_major_vendor_version", {
          major: nameProps.java.major,
          vendor: nameProps.java.vendorName,
          version: nameProps.java.version
        })}
      </span>
      <Show when={hints().get(nameProps.java.id)}>
        {(hint) => (
          <span class="text-lightSlate-700 shrink-0 text-xs">{hint()}</span>
        )}
      </Show>
    </span>
  )

  /** Where the Java comes from: "Installed by GDLauncher · 48 MB", or its
   *  source and path. */
  const Source = (sourceProps: { java: FEJavaEntry }) => {
    const line = () => sourceLine(sourceProps.java, t, size)
    return (
      <span class="text-lightSlate-700 truncate text-xs">
        {line().text}
        <Show when={line().path}>
          {(path) => ` · ${middleTruncate(path(), PATH_MAX)}`}
        </Show>
      </span>
    )
  }

  return (
    <Select<string>
      value={props.value}
      options={props.javas.map((e) => e.id)}
      optionDisabled={(id) => {
        const e = java(id)
        return !e || !e.isValid || e.pendingRemoval
      }}
      // Kobalte also reports the current value once the options arrive.
      onChange={(id) => {
        if (id && id !== props.value) props.onChange(id)
      }}
      disabled={props.disabled}
      placeholder={
        <span class="text-lightSlate-700">{t("java:_trn_choose_a_java")}</span>
      }
      itemComponent={(itemProps) => (
        <SelectItem item={itemProps.item}>
          <Show when={java(itemProps.item.rawValue)}>
            {(e) => (
              <span class="flex min-w-0 flex-col text-left">
                <Name java={e()} />
                <Source java={e()} />
              </span>
            )}
          </Show>
        </SelectItem>
      )}
    >
      <SelectTrigger
        aria-label={props.label}
        class={`gap-2 ${props.class ?? ""}`}
      >
        <SelectValue<string> class="min-w-0">
          {(state) => (
            <Show when={java(state.selectedOption())}>
              {(e) => <Name java={e()} />}
            </Show>
          )}
        </SelectValue>
      </SelectTrigger>
      <SelectContent />
    </Select>
  )
}

export default JavaSelect
