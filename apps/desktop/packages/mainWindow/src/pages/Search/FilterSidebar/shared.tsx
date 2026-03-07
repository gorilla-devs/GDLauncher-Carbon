import { Trans } from "@gd/i18n"

export function FilterWarning() {
  return (
    <div class="mb-2 rounded-md border border-yellow-600/30 bg-yellow-900/20 px-3 py-2">
      <div class="flex items-start gap-2 text-sm text-yellow-200">
        <div class="i-hugeicons:alert-01 mt-0.5 h-4 w-4 shrink-0 text-yellow-500" />
        <span class="leading-relaxed">
          <Trans key="search:_trn_instance_compatibility_warning" />
        </span>
      </div>
    </div>
  )
}
