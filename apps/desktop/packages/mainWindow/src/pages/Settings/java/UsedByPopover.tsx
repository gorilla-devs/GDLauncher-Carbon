import { useTransContext } from "@gd/i18n"
import { Popover, PopoverContent, PopoverTrigger } from "@gd/ui"
import type { FEJavaUsage } from "@gd/core_module/bindings"
import { For, Match, Show, Switch } from "solid-js"
import { useGDNavigate } from "@/managers/NavigationManager"
import { firstNames } from "@/utils/java/javaText"
import IconButton from "./IconButton"
import { textLinkClass } from "@/components/TextLink"

/** "Used by N instances", opening the instances, each linked to its
 *  settings. Instances set to this Java directly carry a settings icon;
 *  those that use it through a custom profile name the profile. */
const UsedByPopover = (props: { usedBy: FEJavaUsage[] }) => {
  const [t] = useTransContext()
  const navigator = useGDNavigate()

  const label = () =>
    t("java:_trn_used_by_instances", { count: props.usedBy.length })
  const shown = () => firstNames(props.usedBy).shown
  const hidden = () => firstNames(props.usedBy).hidden
  const settingsPath = (usage: FEJavaUsage) =>
    `/library/${usage.instanceId}/settings`

  return (
    <Popover placement="bottom-end">
      <PopoverTrigger class={`${textLinkClass("muted")} whitespace-nowrap`}>
        {label()}
      </PopoverTrigger>
      <PopoverContent aria-label={label()} class="w-80" hideCloseButton>
        <ul class="m-0 flex list-none flex-col gap-1 p-0">
          <For each={shown()}>
            {(usage) => (
              <li class="flex min-w-0 items-center gap-1">
                <a
                  href={`#${settingsPath(usage)}`}
                  class="text-lightSlate-100 hover:text-lightSlate-50 truncate rounded text-sm underline-offset-2 outline-none hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
                  onClick={(e) => {
                    e.preventDefault()
                    navigator.navigate(settingsPath(usage))
                  }}
                >
                  {usage.name}
                </a>
                <Switch>
                  <Match when={usage.viaProfile}>
                    {(profile) => (
                      <span class="text-lightSlate-700 min-w-0 shrink-[3] truncate text-xs">
                        {t("java:_trn_via_profile", { profile: profile() })}
                      </span>
                    )}
                  </Match>
                  <Match when={usage.direct}>
                    <IconButton
                      label={t("java:_trn_set_in_instance")}
                      tooltip={t("java:_trn_set_in_instance")}
                    >
                      <div class="i-hugeicons:settings-02 h-4 w-4 text-lightSlate-700" />
                    </IconButton>
                  </Match>
                </Switch>
              </li>
            )}
          </For>
          <Show when={hidden() > 0}>
            <li class="text-lightSlate-700 text-sm">
              {t("java:_trn_and_more", { count: hidden() })}
            </li>
          </Show>
        </ul>
      </PopoverContent>
    </Popover>
  )
}

export default UsedByPopover
