import { useMatch } from "@solidjs/router"
import { createMemo } from "solid-js"
import { AccountStatus, AccountType } from "@gd/core_module/bindings"
import { apiUrl } from "@/utils/rspcClient"
import { getAccountImageUuid } from "@/utils/showcaseHelpers"
import { useGlobalStore } from "./GlobalStoreContext"

export interface AccountsStatus {
  label: {
    name: string
    icon: string | undefined
    uuid: string
    type: AccountType
    status: AccountStatus | undefined
  }
  key: string
}

/** What the top bar and the rail both show: the route's tab and the accounts. */
export function useNavState(): {
  isLogin: () => boolean
  selectedValue: () => "settings" | "news" | ""
  accounts: () => AccountsStatus[]
} {
  const globalStore = useGlobalStore()

  const login = useMatch(() => "/")
  const isSettings = useMatch(() => "/settings")
  const isSettingsNested = useMatch(() => "/settings/*")
  const isNews = useMatch(() => "/news/*")
  const selectedValue = () => {
    if (isSettings() || isSettingsNested()) return "settings"
    if (isNews()) return "news"
    return ""
  }

  const accounts = createMemo(() => {
    return (
      globalStore.accounts.data?.map((account) => {
        const accountStatusQuery = {} as any

        return {
          label: {
            name: account?.username,
            icon: apiUrl(
              `/account/headImage?uuid=${getAccountImageUuid(account)}`
            ),
            uuid: account.uuid,
            type: account.type,
            status: accountStatusQuery.data
          },
          key: account?.uuid
        }
      }) ?? []
    )
  })

  return { isLogin: () => !!login(), selectedValue, accounts }
}
