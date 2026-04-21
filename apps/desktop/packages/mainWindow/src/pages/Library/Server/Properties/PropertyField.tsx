import { Show } from "solid-js"
import {
  Input,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue
} from "@gd/ui"
import type { PropertyDefinition } from "./propertyDefinitions"

interface PropertyFieldProps {
  definition: PropertyDefinition
  value: string
  onChange: (value: string) => void
}

const PropertyField = (props: PropertyFieldProps) => {
  return (
    <div class="flex flex-col gap-1">
      <div class="flex items-center justify-between">
        <label class="text-xs text-lightSlate-500">
          {props.definition.label}
        </label>
        <Show when={props.definition.description}>
          <span class="text-xs text-lightSlate-700">
            {props.definition.description}
          </span>
        </Show>
      </div>

      <Show when={props.definition.type === "boolean"}>
        <button
          class="flex w-fit items-center gap-2 rounded-lg px-3 py-2 text-xs transition-colors"
          classList={{
            "bg-green-900/30 text-green-400": props.value === "true",
            "bg-darkSlate-700 text-lightSlate-500": props.value !== "true"
          }}
          onClick={() =>
            props.onChange(props.value === "true" ? "false" : "true")
          }
        >
          <div
            classList={{
              "i-hugeicons:tick-02": props.value === "true",
              "i-hugeicons:cancel-01": props.value !== "true"
            }}
            class="h-4 w-4"
          />
          {props.value === "true" ? "Enabled" : "Disabled"}
        </button>
      </Show>

      <Show when={props.definition.type === "string"}>
        <Input
          value={props.value}
          onInput={(e) => props.onChange(e.currentTarget.value)}
        />
      </Show>

      <Show when={props.definition.type === "number"}>
        <Input
          type="number"
          value={props.value}
          onInput={(e) => {
            const val = e.currentTarget.value
            if (
              props.definition.min !== undefined &&
              Number(val) < props.definition.min
            )
              return
            if (
              props.definition.max !== undefined &&
              Number(val) > props.definition.max
            )
              return
            props.onChange(val)
          }}
        />
      </Show>

      <Show
        when={props.definition.type === "enum" && props.definition.enumValues}
      >
        <Select
          value={props.value}
          onChange={(value) => {
            if (value) props.onChange(value)
          }}
          options={props.definition.enumValues!}
          disallowEmptySelection={true}
          itemComponent={(itemProps) => (
            <SelectItem item={itemProps.item}>
              {itemProps.item.rawValue}
            </SelectItem>
          )}
        >
          <SelectTrigger>
            <SelectValue<string>>
              {(state) => state.selectedOption()}
            </SelectValue>
          </SelectTrigger>
          <SelectContent />
        </Select>
      </Show>
    </div>
  )
}

export default PropertyField
