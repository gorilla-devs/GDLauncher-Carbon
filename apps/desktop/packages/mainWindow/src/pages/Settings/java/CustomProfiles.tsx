import { useTransContext } from "@gd/i18n"
import { Button, Tooltip, TooltipContent, TooltipTrigger } from "@gd/ui"
import type { FEJavaOverview, FEJavaProfile } from "@gd/core_module/bindings"
import { For, Show, createUniqueId } from "solid-js"
import { useModal } from "@/managers/ModalsManager"
import type { DeleteJavaProfileConfirmData } from "@/managers/ModalsManager/modals/Java/DeleteJavaProfileConfirm"
import { rspc } from "@/utils/rspcClient"
import { customProfileUsers, javaChoices } from "./javaOverviewView"
import IconButton from "./IconButton"
import JavaSelect from "@/components/Java/JavaSelect"

interface Props {
  profiles: FEJavaProfile[]
  /** Missing while it loads: the selects then have nothing to offer. */
  overview: FEJavaOverview | undefined
}

/** Profiles the user made: a name and a Java. Instances that name one use
 *  its Java. */
const CustomProfiles = (props: Props) => {
  const [t] = useTransContext()
  const modalsContext = useModal()
  const headingId = createUniqueId()
  let heading: HTMLHeadingElement | undefined
  let addButton: HTMLButtonElement | undefined

  const updateProfile = rspc.createMutation(() => ({
    mutationKey: ["java.updateJavaProfile"]
  }))
  const deleteProfile = rspc.createMutation(() => ({
    mutationKey: ["java.deleteJavaProfile"]
  }))

  const custom = () => props.profiles.filter((profile) => !profile.isSystem)
  // Rows are keyed by name: a refetch brings new profile objects, and a row
  // that remounted would drop the focus of the select just used.
  const names = () => custom().map((profile) => profile.name)
  const javaIdOf = (name: string) =>
    custom().find((profile) => profile.name === name)?.javaId ?? null
  const javas = () => (props.overview ? javaChoices(props.overview) : [])
  // Where focus goes when the deleted row's button goes away.
  const focusHeading = () => heading?.focus()

  const create = () =>
    modalsContext?.openModal(
      { name: "javaProfileCreation" },
      { onClosed: () => addButton?.focus() }
    )

  const remove = (name: string, button: HTMLButtonElement | undefined) => {
    if (deleteProfile.isPending) return
    const users = customProfileUsers(props.overview, name)
    if (users === 0) {
      deleteProfile.mutate(name, { onSuccess: focusHeading })
      return
    }

    const data: DeleteJavaProfileConfirmData = {
      name,
      users,
      onConfirm: () => deleteProfile.mutateAsync(name),
      onClosed: (deleted) => (deleted ? focusHeading() : button?.focus())
    }
    modalsContext?.openModal({ name: "deleteJavaProfileConfirm" }, data)
  }

  return (
    <section
      aria-labelledby={headingId}
      class="border-darkSlate-600 rounded-xl border border-solid"
    >
      <div class="flex items-center justify-between gap-4 px-4 py-3">
        <h5
          ref={heading}
          id={headingId}
          tabIndex={-1}
          class="text-lightSlate-50 m-0 rounded text-base font-medium outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
        >
          {t("java:_trn_custom_profiles")}
        </h5>
        <Tooltip>
          <TooltipTrigger
            ref={(el) => (addButton = el)}
            type="button"
            class="b-0 rounded-md bg-transparent p-0 outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-500"
            aria-label={t("java:_trn_create_custom_profile")}
            onClick={create}
          >
            <Button as="div" type="secondary" size="small">
              <div class="i-hugeicons:add-01 text-xl" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            {t("java:_trn_create_custom_profile")}
          </TooltipContent>
        </Tooltip>
      </div>
      <Show when={names().length > 0}>
        <div class="flex flex-col px-2 pb-2">
          <For each={names()}>
            {(name) => {
              let deleteButton: HTMLButtonElement | undefined
              return (
                <div class="flex items-center gap-3 rounded-md px-2 py-2 hover:bg-darkSlate-600">
                  <span class="text-lightSlate-50 min-w-0 flex-1 truncate">
                    {name}
                  </span>
                  <JavaSelect
                    javas={javas()}
                    value={javaIdOf(name)}
                    label={t("java:_trn_a11y_profile_java", { name })}
                    class="w-80"
                    onChange={(javaId) =>
                      updateProfile.mutate({ profileName: name, javaId })
                    }
                  />
                  <IconButton
                    ref={(el) => (deleteButton = el)}
                    label={t("java:_trn_a11y_delete", { name })}
                    tooltip={t("general:_trn_delete")}
                    class="text-lightSlate-700 hover:text-red-400"
                    onClick={() => remove(name, deleteButton)}
                  >
                    <div class="i-hugeicons:delete-02 h-5 w-5" />
                  </IconButton>
                </div>
              )
            }}
          </For>
        </div>
      </Show>
    </section>
  )
}

export default CustomProfiles
