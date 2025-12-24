import { Trans } from "@gd/i18n"
import {
  Button,
  Input,
  Slider,
  Switch as GDSwitch,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
  TabsIndicator,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
  Popover,
  PopoverContent,
  PopoverTrigger,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenu
} from "@gd/ui"
import { useRouteData } from "@solidjs/router"
import { For, Match, Show, Switch, createMemo } from "solid-js"
import SettingsJavaData from "./settings.java.data"
import { useModal } from "@/managers/ModalsManager"
import { rspc } from "@/utils/rspcClient"
import PageTitle from "./components/PageTitle"
import Row from "./components/Row"
import Title from "./components/Title"
import RowsContainer from "./components/RowsContainer"
import RightHandSide from "./components/RightHandSide"
import { generateSequence } from "@/utils/helpers"
import Center from "./components/Center"
import TruncatedPath from "@/components/TruncatePath"

const Java = () => {
  const routeData: ReturnType<typeof SettingsJavaData> = useRouteData()
  const javasData = () => routeData?.availableJavas
  const javas = () => javasData()?.data || []
  const modalsContext = useModal()

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const profileAssignments = rspc.createQuery(() => ({
    queryKey: ["java.systemJavaProfileAssignments"]
  }))

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const updateProfile = rspc.createMutation(() => ({
    mutationKey: ["java.updateJavaProfile"]
  }))

  const deleteProfile = rspc.createMutation(() => ({
    mutationKey: ["java.deleteJavaProfile"]
  }))

  const deleteJavaMutation = rspc.createMutation(() => ({
    mutationKey: ["java.deleteJavaVersion"]
  }))

  const mbTotalRAM = () =>
    Math.round(Number(routeData.totalRam.data) / 1024 / 1024)

  const initialJavaArgs = createMemo((prev: string | undefined) => {
    if (prev) return prev

    return settings.data?.javaCustomArgs
  })

  const flattenedAvailableJavas = () =>
    Object.values(routeData.availableJavas.data || {}).reduce(
      (acc, curr) => acc.concat(curr),
      []
    )

  const availableJavasOptions = () => {
    const results = flattenedAvailableJavas()?.map((java) => java.id) || []
    return ["unassigned", ...results]
  }

  const getJavaById = (id: string) => {
    if (id === "unassigned") return null
    return flattenedAvailableJavas()?.find((java) => java.id === id)
  }

  const renderJavaOption = (id: string) => {
    if (id === "unassigned") {
      return (
        <div>
          <Trans key="java:_trn_unassigned" />
        </div>
      )
    }

    const java = getJavaById(id)
    if (!java) return <div>{id}</div>

    return (
      <div class="flex w-full flex-col gap-2">
        <div class="flex justify-between">
          <div class="text-lightSlate-50">{java.version}</div>
          <div>{java.type}</div>
        </div>
        <div class="w-full text-left">
          <Tooltip>
            <TooltipTrigger>
              <TruncatedPath originalPath={java.path} />
            </TooltipTrigger>
            <TooltipContent>{java.path}</TooltipContent>
          </Tooltip>
        </div>
      </div>
    )
  }

  const javaProfiles = () => [
    (routeData.javaProfiles.data || []).filter((profile) => profile.isSystem),
    (routeData.javaProfiles.data || []).filter((profile) => !profile.isSystem)
  ]

  const menuItems = () => [
    {
      icon: "i-hugeicons:pencil-edit-01",
      label: "Add Managed",
      action: () => {
        modalsContext?.openModal({ name: "addManagedJava" })
      }
    },
    {
      icon: "i-hugeicons:folder-open",
      label: "Add Custom",
      action: () => {
        modalsContext?.openModal({ name: "addCustomJava" })
      }
    }
  ]

  return (
    <>
      <PageTitle>
        <Trans key="settings:_trn_java" />
      </PageTitle>
      <RowsContainer>
        <Row forceContentBelow>
          <Title>
            <Trans key="java:_trn_java_memory_title" />
          </Title>
          <Center>
            <Slider
              min={1024}
              max={mbTotalRAM()}
              steps={1024}
              marks={generateSequence(1024, mbTotalRAM())}
              value={settings.data?.xmx}
              onChange={(val) => {
                settingsMutation.mutate({
                  xmx: {
                    Set: val
                  }
                })
              }}
            />
            <Input
              class="w-26"
              value={settings.data?.xmx}
              onChange={(e) => {
                settingsMutation.mutate({
                  xmx: {
                    Set: parseInt(e.currentTarget.value, 10)
                  }
                })
              }}
            />
          </Center>
        </Row>
        <Row class="flex-col items-stretch">
          <Title>
            <Trans key="java:_trn_java_arguments_title" />
          </Title>
          <div class="flex items-center justify-center gap-4">
            <Input
              class="w-full"
              value={settings.data?.javaCustomArgs}
              onChange={(e) => {
                settingsMutation.mutate({
                  javaCustomArgs: {
                    Set: e.target.value
                  }
                })
              }}
            />
            <Tooltip>
              <TooltipTrigger>
                <Button
                  type="secondary"
                  class="h-10"
                  size="small"
                  onClick={() => {
                    settingsMutation.mutate({
                      javaCustomArgs: {
                        Set: initialJavaArgs() || ""
                      }
                    })
                  }}
                >
                  <div class="i-hugeicons:arrow-turn-backward h-5 w-5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="ui:_trn_tooltip.undo" />
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger>
                <Button
                  type="secondary"
                  class="h-10"
                  size="small"
                  onClick={() => {
                    settingsMutation.mutate({
                      javaCustomArgs: {
                        Set: ""
                      }
                    })
                  }}
                >
                  <div class="i-hugeicons:cancel-01 h-5 w-5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <Trans key="ui:_trn_tooltip.reset" />
              </TooltipContent>
            </Tooltip>
          </div>
        </Row>
        <Row>
          <Title
            description={
              <Trans key="java:_trn_auto_manage_java_system_profiles_text" />
            }
          >
            <Trans key="java:_trn_auto_manage_java_system_profiles" />
          </Title>
          <RightHandSide>
            <GDSwitch
              checked={settings.data?.autoManageJavaSystemProfiles}
              onChange={(e) => {
                settingsMutation.mutate({
                  autoManageJavaSystemProfiles: {
                    Set: e.target.checked
                  }
                })
              }}
            />
          </RightHandSide>
        </Row>
        <div class="flex flex-col">
          <div class="overflow-hidden">
            <Tabs defaultValue="paths">
              <TabsList class="w-full">
                <TabsIndicator />
                <TabsTrigger value="paths" class="flex-1">
                  <Trans key="java:_trn_manage_paths" />
                </TabsTrigger>
                <TabsTrigger value="profiles" class="flex-1">
                  <Trans key="java:_trn_manage_profiles" />
                </TabsTrigger>
              </TabsList>
              <TabsContent value="paths">
                <div class="h-full min-h-96 p-4">
                  <div class="mb-4 flex items-center justify-between">
                    <div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java:_trn_java_description_text" />
                      </div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java:_trn_java_description_local_text" />
                      </div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java:_trn_java_description_managed_text" />
                      </div>
                      <div class="m-0 text-sm font-normal">
                        <Trans key="java:_trn_java_description_custom_text" />
                      </div>
                    </div>
                    <DropdownMenu placement="bottom-end">
                      <DropdownMenuTrigger class="b-0 bg-transparent p-0">
                        <Button as="div" type="secondary" size="small">
                          <div class="i-hugeicons:add-01 text-xl" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent>
                        <For each={menuItems()}>
                          {(item) => (
                            <DropdownMenuItem onSelect={item.action}>
                              <div class="flex items-center gap-2">
                                <i class={item.icon} />
                                <span>{item.label}</span>
                              </div>
                            </DropdownMenuItem>
                          )}
                        </For>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                  <div class="flex flex-col gap-4">
                    <For each={Object.entries(javas())}>
                      {([javaVersion, obj]) => (
                        <div class="border-1 border-darkSlate-600 rounded-xl border-solid">
                          <h3 class="px-4">
                            <Trans
                              key="java:_trn_java_version_number"
                              options={{
                                version: javaVersion
                              }}
                            />
                          </h3>
                          <Show when={obj.length > 0}>
                            <div class="flex flex-col gap-2">
                              <For each={obj}>
                                {(java) => {
                                  const usedInNProfiles = () =>
                                    (routeData.javaProfiles.data || []).filter(
                                      (item) => item.javaId === java.id
                                    )

                                  return (
                                    <div class="hover:bg-darkSlate-600 flex justify-between rounded-md px-4 py-2">
                                      <div class="flex w-full min-w-0 flex-1 flex-col gap-2">
                                        <div class="flex justify-between">
                                          <div class="flex items-center gap-2">
                                            <div class="text-lightSlate-50 flex items-center gap-2">
                                              <div>{java.version}</div>
                                              <Switch>
                                                <Match when={java.isValid}>
                                                  <Tooltip>
                                                    <TooltipTrigger>
                                                      <div class="i-hugeicons:tick-double-02 flex text-emerald-500 text-lg" />
                                                    </TooltipTrigger>
                                                    <TooltipContent>
                                                      <Trans key="java:_trn_tooltip_valid_path" />
                                                    </TooltipContent>
                                                  </Tooltip>
                                                </Match>
                                                <Match when={!java.isValid}>
                                                  <Tooltip>
                                                    <TooltipTrigger>
                                                      <div class="i-hugeicons:alert-02 flex text-yellow-500 text-lg" />
                                                    </TooltipTrigger>
                                                    <TooltipContent>
                                                      <Trans key="java:_trn_tooltip_invalid_path" />
                                                    </TooltipContent>
                                                  </Tooltip>
                                                </Match>
                                              </Switch>
                                            </div>
                                            <Show
                                              when={
                                                usedInNProfiles().length > 0
                                              }
                                            >
                                              <div class="bg-darkSlate-400 mr-2 h-2/3 w-px" />
                                              <Popover>
                                                <PopoverTrigger>
                                                  <div class="text-sm underline">
                                                    <Trans
                                                      key="java:_trn_used_in_counted_profiles"
                                                      options={{
                                                        count:
                                                          usedInNProfiles()
                                                            .length
                                                      }}
                                                    />
                                                  </div>
                                                </PopoverTrigger>
                                                <PopoverContent>
                                                  <div class="p-4">
                                                    <h3>
                                                      <Trans key="java:_trn_used_in_the_following_profiles" />
                                                    </h3>
                                                    <ul class="flex flex-col gap-2">
                                                      <For
                                                        each={usedInNProfiles()}
                                                      >
                                                        {(profile) => (
                                                          <li class="text-lightSlate-600">
                                                            {profile.name}
                                                          </li>
                                                        )}
                                                      </For>
                                                    </ul>
                                                  </div>
                                                </PopoverContent>
                                              </Popover>
                                            </Show>
                                          </div>
                                        </div>
                                        <div class="flex justify-between">
                                          <div class="text-lightSlate-700 flex-1 overflow-hidden whitespace-nowrap text-xs">
                                            <Tooltip>
                                              <TooltipTrigger>
                                                <TruncatedPath
                                                  originalPath={java.path}
                                                />
                                              </TooltipTrigger>
                                              <TooltipContent>
                                                {java.path}
                                              </TooltipContent>
                                            </Tooltip>
                                          </div>
                                        </div>
                                      </div>
                                      <div class="ml-2 flex items-center">
                                        <Show
                                          when={
                                            java.type === "custom" ||
                                            java.type === "managed"
                                          }
                                        >
                                          <div
                                            class="i-hugeicons:delete-02 text-lightSlate-700 text-lg transition-color duration-100 ease-spring hover:text-red-400"
                                            onClick={() =>
                                              deleteJavaMutation.mutate(java.id)
                                            }
                                          />
                                        </Show>
                                      </div>
                                    </div>
                                  )
                                }}
                              </For>
                            </div>
                          </Show>
                          <Show when={obj.length === 0}>
                            <p>
                              <Trans key="java:_trn_no_found_java_text" />
                            </p>
                          </Show>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </TabsContent>
              <TabsContent value="profiles">
                <div class="flex h-full min-h-96 flex-col gap-4 p-4">
                  <div class="mb-4 flex items-center justify-between">
                    <h2 class="m-0 text-sm font-normal">
                      <Trans key="java:_trn_profiles_description_text" />
                    </h2>
                  </div>
                  <For each={javaProfiles()}>
                    {(profiles, i) => (
                      <div class="border-1 border-darkSlate-600 rounded-xl border-solid">
                        <div class="flex items-center justify-between px-4">
                          <h3>
                            <Switch>
                              <Match when={i() === 0}>
                                <Trans key="java:_trn_system_profiles" />
                              </Match>
                              <Match when={i() === 1}>
                                <Trans key="java:_trn_custom_profiles" />
                              </Match>
                            </Switch>
                          </h3>
                          <Show when={i() === 1}>
                            <Button
                              type="secondary"
                              size="small"
                              onClick={() => {
                                modalsContext?.openModal({
                                  name: "javaProfileCreation"
                                })
                              }}
                            >
                              <div class="i-hugeicons:add-01 text-xl" />
                            </Button>
                          </Show>
                        </div>
                        <For each={profiles}>
                          {(profile) => {
                            const id = flattenedAvailableJavas()?.find(
                              (java) => java.id === profile.javaId
                            )?.id

                            return (
                              <div class="hover:bg-darkSlate-600 flex items-center justify-between px-4 py-2">
                                <h3 class="text-lightSlate-700 m-0 text-sm">
                                  {profile.name}
                                </h3>
                                <div class="m-0 flex items-center gap-4">
                                  <Select
                                    value={id || "unassigned"}
                                    options={availableJavasOptions()}
                                    disabled={
                                      profile.isSystem &&
                                      settings.data
                                        ?.autoManageJavaSystemProfiles
                                    }
                                    onChange={(value) => {
                                      if (value) {
                                        updateProfile.mutate({
                                          profileName: profile.name,
                                          javaId:
                                            value === "unassigned"
                                              ? null
                                              : value
                                        })
                                      }
                                    }}
                                    itemComponent={(props) => (
                                      <SelectItem item={props.item}>
                                        {renderJavaOption(props.item.rawValue)}
                                      </SelectItem>
                                    )}
                                  >
                                    <SelectTrigger class="w-70">
                                      <SelectValue<string>>
                                        {(state) =>
                                          renderJavaOption(
                                            state.selectedOption() ||
                                              "unassigned"
                                          )
                                        }
                                      </SelectValue>
                                    </SelectTrigger>
                                    <SelectContent />
                                  </Select>
                                  <Show when={i() === 1}>
                                    <div
                                      class="i-hugeicons:delete-02 text-lightSlate-700 text-lg transition-color duration-100 ease-spring hover:text-red-400"
                                      onClick={() => {
                                        deleteProfile.mutate(profile.name)
                                      }}
                                    />
                                  </Show>
                                  <Show when={profile.isSystem}>
                                    <Popover>
                                      <PopoverTrigger>
                                        <div class="i-hugeicons:information-circle text-lightSlate-700 hover:text-lightSlate-100 text-lg transition-color duration-100 ease-spring" />
                                      </PopoverTrigger>
                                      <PopoverContent>
                                        <div class="max-w-100 flex h-auto flex-col gap-8 p-4">
                                          <div>
                                            <Trans key="java:_trn_profile_used_in_mc_versions" />
                                          </div>
                                          <div class="h-70 flex flex-wrap content-start items-start justify-start gap-4 overflow-y-auto p-4">
                                            <For
                                              each={
                                                profileAssignments.data?.[
                                                  profile.name
                                                ] || []
                                              }
                                            >
                                              {(assignment) => (
                                                <div class="text-lightSlate-700">
                                                  {assignment}
                                                </div>
                                              )}
                                            </For>
                                          </div>
                                        </div>
                                      </PopoverContent>
                                    </Popover>
                                  </Show>
                                </div>
                              </div>
                            )
                          }}
                        </For>
                      </div>
                    )}
                  </For>
                </div>
              </TabsContent>
            </Tabs>
          </div>
        </div>
      </RowsContainer>
    </>
  )
}

export default Java
