import { rspc } from "@/utils/rspcClient"
import {
  AccountEntry,
  Announcement,
  FEGDLAccountStatus,
  FESettings,
  FEUnifiedCategories,
  FEUnifiedModLoaders,
  ListGroup,
  ListInstance,
  ListServer,
  ListServerGroup,
  ManifestVersion
} from "@gd/core_module/bindings"
import { RSPCError } from "@/utils/rspcClient"
import { UseQueryResult } from "@tanstack/solid-query"
import {
  JSX,
  createContext,
  useContext,
  createSignal,
  createEffect
} from "solid-js"

interface Context {
  instances: UseQueryResult<ListInstance[], RSPCError>
  instanceGroups: UseQueryResult<ListGroup[], RSPCError>
  servers: UseQueryResult<ListServer[], RSPCError>
  serverGroups: UseQueryResult<ListServerGroup[], RSPCError>
  settings: UseQueryResult<FESettings, RSPCError>
  accounts: UseQueryResult<AccountEntry[], RSPCError>
  currentlySelectedAccount: () => AccountEntry | null
  currentlySelectedAccountUuid: UseQueryResult<string | null, RSPCError>
  gdlAccount: UseQueryResult<FEGDLAccountStatus | null, RSPCError>
  announcements: UseQueryResult<Announcement[], RSPCError>
  categories: UseQueryResult<FEUnifiedCategories, RSPCError>
  modloaders: UseQueryResult<FEUnifiedModLoaders, RSPCError>
  minecraftVersions: UseQueryResult<ManifestVersion[], RSPCError>
  isNewInstance: (id: number) => boolean
  markInstanceAsSeen: (id: number) => void
}

const GlobalStoreContext = createContext()

export const GlobalStoreProvider = (props: { children: JSX.Element }) => {
  const instances = rspc.createQuery(() => ({
    queryKey: ["instance.getAllInstances"]
  }))

  const groups = rspc.createQuery(() => ({
    queryKey: ["instance.getGroups"]
  }))

  const settings = rspc.createQuery(() => ({
    queryKey: ["settings.getSettings"]
  }))

  const accountsRaw = rspc.createQuery(() => ({
    queryKey: ["account.getAccounts"]
  }))

  const currentlySelectedAccountUuidRaw = rspc.createQuery(() => ({
    queryKey: ["account.getActiveUuid"]
  }))

  const gdlAccountRaw = rspc.createQuery(() => ({
    queryKey: ["account.getGdlAccount"]
  }))

  // Map real UUID to anonymized UUID in showcase mode
  const currentlySelectedAccountUuid = __SHOWCASE_MODE__
    ? // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-assertion
      (new Proxy(currentlySelectedAccountUuidRaw, {
        get(target, prop) {
          if (prop === "data") {
            const realUuid = target.data
            if (!realUuid) return null

            // Find the index of the account with this UUID in the raw data
            const index = accountsRaw.data?.findIndex(
              (account) => account.uuid === realUuid
            )
            if (index !== undefined && index !== -1) {
              return `00000000-0000-0000-0000-00000000000${index}`
            }
            return realUuid
          }
          return target[prop as keyof typeof target]
        }
      }) as UseQueryResult<string | null, RSPCError>)
    : currentlySelectedAccountUuidRaw

  // In showcase mode, create reactive proxy objects that anonymize data
  const accounts = __SHOWCASE_MODE__
    ? // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-assertion
      (new Proxy(accountsRaw, {
        get(target, prop) {
          if (prop === "data") {
            return target.data?.map((account, index) => ({
              ...account,
              username: "Player",
              uuid: `00000000-0000-0000-0000-00000000000${index}`,
              // Store real UUID for image fetching (prefixed with underscore to indicate internal use)
              _realUuid: account.uuid,
              type:
                account.type.type === "microsoft"
                  ? {
                      type: "microsoft" as const,
                      value: { email: "player@example.com" }
                    }
                  : account.type
            }))
          }
          return target[prop as keyof typeof target]
        }
      }) as UseQueryResult<AccountEntry[], RSPCError>)
    : accountsRaw

  const gdlAccount = __SHOWCASE_MODE__
    ? // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-assertion
      (new Proxy(gdlAccountRaw, {
        get(target, prop) {
          if (prop === "data") {
            const data = target.data
            if (data?.status === "valid") {
              return {
                ...data,
                value: {
                  ...data.value,
                  displayName: "DemoUser",
                  email: "demo@example.com",
                  friendCode: "DEMO#0000",
                  microsoftOid: "00000000-0000-0000-0000-000000000000",
                  microsoftEmail: "player@example.com"
                }
              }
            }
            return data
          }
          return target[prop as keyof typeof target]
        }
      }) as UseQueryResult<FEGDLAccountStatus | null, RSPCError>)
    : gdlAccountRaw

  const currentlySelectedAccount = () => {
    const uuid = currentlySelectedAccountUuid.data
    if (!uuid) return null

    return accounts.data?.find((account) => account.uuid === uuid) || null
  }

  const announcements = rspc.createQuery(() => ({
    queryKey: ["getAnnouncements"]
  }))

  const categories = rspc.createQuery(() => ({
    queryKey: ["modplatforms.getUnifiedCategories"]
  }))

  const modloaders = rspc.createQuery(() => ({
    queryKey: ["modplatforms.getUnifiedModloaders"]
  }))

  const minecraftVersions = rspc.createQuery(() => ({
    queryKey: ["mc.getMinecraftVersions"]
  }))

  const servers = rspc.createQuery(() => ({
    queryKey: ["server.getAllServers"]
  }))

  const serverGroups = rspc.createQuery(() => ({
    queryKey: ["server.getGroups"]
  }))

  // Track which instances existed at app load (for NEW badge feature)
  const [baselineInstanceIds, setBaselineInstanceIds] = createSignal<
    Set<number>
  >(new Set())
  const [baselineInitialized, setBaselineInitialized] = createSignal(false)

  // Track which instances user has interacted with
  const [seenInstanceIds, setSeenInstanceIds] = createSignal<Set<number>>(
    new Set()
  )

  // Capture baseline on first successful instances load
  createEffect(() => {
    if (!baselineInitialized() && instances.data) {
      setBaselineInstanceIds(new Set(instances.data.map((i) => i.id)))
      setBaselineInitialized(true)
    }
  })

  const isNewInstance = (id: number): boolean => {
    if (!baselineInitialized()) return false
    return !baselineInstanceIds().has(id) && !seenInstanceIds().has(id)
  }

  const markInstanceAsSeen = (id: number): void => {
    setSeenInstanceIds((prev) => new Set([...prev, id]))
  }

  const store: Context = {
    instances,
    instanceGroups: groups,
    servers,
    serverGroups,
    settings,
    accounts,
    currentlySelectedAccountUuid,
    currentlySelectedAccount,
    gdlAccount,
    announcements,
    categories,
    modloaders,
    minecraftVersions,
    isNewInstance,
    markInstanceAsSeen
  }

  return (
    <GlobalStoreContext.Provider value={store}>
      {props.children}
    </GlobalStoreContext.Provider>
  )
}

export const useGlobalStore = (): Context => {
  return useContext(GlobalStoreContext) as Context
}
