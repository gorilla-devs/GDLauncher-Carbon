import { useTransContext } from "@gd/i18n"
import {
  Button,
  Popover,
  PopoverContent,
  PopoverTrigger,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  toast
} from "@gd/ui"
import type {
  FEAdvisoryFix,
  FEJavaEntry,
  FEJavaKnownIssue
} from "@gd/core_module/bindings"
import { For, Show } from "solid-js"
import { textLinkClass } from "@/components/TextLink"
import { rspc } from "@/utils/rspcClient"
import {
  isUpdatingManagedJava,
  updateManagedJava
} from "@/utils/managedJavaUpdates"
import { securityFlawText } from "@/utils/java/javaText"
import { ICON_BUTTON_CLASS } from "./IconButton"

interface Props {
  entry: FEJavaEntry
  name: string
  /** A found or added Java nothing uses: the icon is muted and the popover
   *  says so rather than offering a fix. */
  muted: boolean
  /** Controlled by the row, so the Review landing can open it. */
  open: boolean
  onOpenChange: (open: boolean) => void
  triggerRef?: (el: HTMLButtonElement) => void
}

/** The known-issue icon of a row. Its tooltip names the issues; its popover
 *  says each is a security flaw, links its details, says whether a fixed
 *  build exists and whether launches are protected meanwhile, and offers
 *  the one fix that fits. */
const AdvisoryPopover = (props: Props) => {
  const [t, { getI18next }] = useTransContext()
  let content: HTMLDivElement | undefined

  const switchJava = rspc.createMutation(() => ({
    mutationKey: ["java.switchJavaToFixed"]
  }))

  /** "Security flaw: Java4Shell (CVE-2026-114938) · Details". The id, the
   *  separator and the link stay on one line: the id would break at its
   *  hyphens, and a lone "Details" or a dangling "·" reads as a mistake. */
  const flawLine = (issue: FEJavaKnownIssue) => {
    const marker = "\u0000"
    const [before, after = ""] = t("java:_trn_security_flaw", {
      name: marker
    }).split(marker)
    const named = issue.name !== issue.advisoryId
    return (
      <>
        {before}
        {named ? `${issue.name} ` : ""}
        <span class="whitespace-nowrap">
          {named ? `(${issue.advisoryId})` : issue.advisoryId}
          {after}
          <span class="text-darkSlate-400"> · </span>
          <a
            href={issue.url}
            aria-label={t("java:_trn_a11y_details", { name: issue.name })}
            class={textLinkClass("muted")}
            onClick={(e) => {
              e.preventDefault()
              window.openExternalLink(issue.url)
            }}
          >
            {t("java:_trn_details")}
          </a>
        </span>
      </>
    )
  }

  const fixLabel = (fix: FEAdvisoryFix) =>
    "update" in fix
      ? t("java:_trn_update_to_version", { version: fix.update.version })
      : t("java:_trn_switch_to_azul", { version: fix.switchToAzul.version })

  const fixBusy = (fix: FEAdvisoryFix) =>
    "update" in fix
      ? isUpdatingManagedJava(props.entry.id)
      : switchJava.isPending

  // A Java on its way out needs no fix, nor does one nothing uses.
  const fixOf = (issue: FEJavaKnownIssue) =>
    props.entry.pendingRemoval || props.muted ? null : issue.fix

  const applyFix = (fix: FEAdvisoryFix) => {
    if ("update" in fix) {
      void updateManagedJava(props.entry.id, fix.update.version)
      return
    }
    const { buildId, version } = fix.switchToAzul
    switchJava.mutate(
      { javaId: props.entry.id, buildId },
      {
        onSuccess: () => {
          props.onOpenChange(false)
          toast.success(t("java:_trn_switched_to_azul", { version }))
        }
      }
    )
  }

  // The fix is what the user came for; a link first would send Enter to a
  // web page.
  const focusFix = (e: Event) => {
    e.preventDefault()
    const fix = content?.querySelector<HTMLElement>("[data-advisory-fix]")
    ;(fix ?? content)?.focus()
  }

  return (
    <Popover
      open={props.open}
      onOpenChange={props.onOpenChange}
      placement="bottom-start"
    >
      {/* The popover says more, and a tooltip left open under the pointer
          would cover it. */}
      <Tooltip open={props.open ? false : undefined}>
        <PopoverTrigger
          as={TooltipTrigger}
          ref={props.triggerRef}
          aria-label={t("java:_trn_a11y_security_flaw", { name: props.name })}
          class={ICON_BUTTON_CLASS}
        >
          <div
            class="i-hugeicons:alert-diamond h-4 w-4"
            classList={{
              "text-yellow-500": !props.muted,
              "text-lightSlate-700": props.muted
            }}
          />
        </PopoverTrigger>
        <TooltipContent>
          {securityFlawText(
            props.entry.knownIssues.map((issue) => issue.name),
            getI18next().language,
            t
          )}
        </TooltipContent>
      </Tooltip>
      <PopoverContent
        ref={content}
        aria-label={t("java:_trn_a11y_security_flaw", { name: props.name })}
        class="w-80"
        hideCloseButton
        onOpenAutoFocus={focusFix}
      >
        <div class="flex flex-col gap-3">
          <For each={props.entry.knownIssues}>
            {(issue) => (
              <div class="flex flex-col items-start gap-1">
                <p class="text-lightSlate-50 m-0 text-sm">{flawLine(issue)}</p>
                <p class="text-lightSlate-700 m-0 text-sm">
                  <Show
                    when={issue.fixedVersion}
                    fallback={t("java:_trn_no_fixed_build")}
                  >
                    {(version) =>
                      t("java:_trn_fixed_in_version", { version: version() })
                    }
                  </Show>
                </p>
                <Show when={issue.mitigated && !props.muted}>
                  <p class="text-lightSlate-700 m-0 text-sm">
                    {t("java:_trn_protected_until_update")}
                  </p>
                </Show>
                <Show when={fixOf(issue)}>
                  {(fix) => (
                    <div class="flex w-full justify-end pt-2">
                      <Button
                        type="secondary"
                        size="small"
                        data-advisory-fix
                        loading={fixBusy(fix())}
                        onClick={() => applyFix(fix())}
                      >
                        {fixLabel(fix())}
                      </Button>
                    </div>
                  )}
                </Show>
              </div>
            )}
          </For>
          <Show when={props.muted}>
            <p class="text-lightSlate-300 m-0 text-sm">
              {t("java:_trn_nothing_uses_it")}
            </p>
          </Show>
        </div>
      </PopoverContent>
    </Popover>
  )
}

export default AdvisoryPopover
