import { createSignal, For, Show } from "solid-js"
import { useParams } from "@solidjs/router"
import {
  Button,
  Input,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
  TabsIndicator,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue
} from "@gd/ui"
import { Trans, useTransContext, type NamespacedTranslationKey } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"

const OP_LEVEL_KEYS: Record<string, string> = {
  "1": "instances:_trn_server_players_op_level_1",
  "2": "instances:_trn_server_players_op_level_2",
  "3": "instances:_trn_server_players_op_level_3",
  "4": "instances:_trn_server_players_op_level_4"
}

const PlayersTab = () => {
  const [t] = useTransContext()
  const params = useParams()
  const serverId = () => parseInt(params.id!, 10)

  const opLevelLabel = (level: string) =>
    OP_LEVEL_KEYS[level]
      ? t(OP_LEVEL_KEYS[level] as NamespacedTranslationKey)
      : level

  const [activeList, setActiveList] = createSignal("whitelist")
  const [newUsername, setNewUsername] = createSignal("")
  const [opLevel, setOpLevel] = createSignal("4")
  const [banReason, setBanReason] = createSignal("")
  const [banIp, setBanIp] = createSignal("")

  const whitelistQuery = rspc.createQuery(() => ({
    queryKey: ["server.getWhitelist", serverId()]
  }))

  const opsQuery = rspc.createQuery(() => ({
    queryKey: ["server.getOps", serverId()]
  }))

  const bannedQuery = rspc.createQuery(() => ({
    queryKey: ["server.getBannedPlayers", serverId()]
  }))

  const bannedIpsQuery = rspc.createQuery(() => ({
    queryKey: ["server.getBannedIps", serverId()]
  }))

  const addWhitelistMutation = rspc.createMutation(() => ({
    mutationKey: ["server.addToWhitelist"]
  }))

  const removeWhitelistMutation = rspc.createMutation(() => ({
    mutationKey: ["server.removeFromWhitelist"]
  }))

  const addOpMutation = rspc.createMutation(() => ({
    mutationKey: ["server.addOp"]
  }))

  const removeOpMutation = rspc.createMutation(() => ({
    mutationKey: ["server.removeOp"]
  }))

  const banPlayerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.banPlayer"]
  }))

  const unbanPlayerMutation = rspc.createMutation(() => ({
    mutationKey: ["server.unbanPlayer"]
  }))

  const banIpMutation = rspc.createMutation(() => ({
    mutationKey: ["server.banIp"]
  }))

  const unbanIpMutation = rspc.createMutation(() => ({
    mutationKey: ["server.unbanIp"]
  }))

  const handleAddWhitelist = async () => {
    const name = newUsername().trim()
    if (!name) return
    await addWhitelistMutation.mutateAsync({
      serverId: serverId(),
      username: name
    })
    setNewUsername("")
    whitelistQuery.refetch()
  }

  const handleAddOp = async () => {
    const name = newUsername().trim()
    if (!name) return
    await addOpMutation.mutateAsync({
      serverId: serverId(),
      username: name,
      level: parseInt(opLevel(), 10)
    })
    setNewUsername("")
    opsQuery.refetch()
  }

  const handleBanPlayer = async () => {
    const name = newUsername().trim()
    if (!name) return
    await banPlayerMutation.mutateAsync({
      serverId: serverId(),
      username: name,
      reason: banReason() || null
    })
    setNewUsername("")
    setBanReason("")
    bannedQuery.refetch()
  }

  const handleBanIp = async () => {
    const ip = banIp().trim()
    if (!ip) return
    await banIpMutation.mutateAsync({
      serverId: serverId(),
      ip,
      reason: banReason() || null
    })
    setBanIp("")
    setBanReason("")
    bannedIpsQuery.refetch()
  }

  return (
    <div class="h-full w-full overflow-y-auto">
      <Tabs value={activeList()} onChange={(v) => setActiveList(v)}>
        <TabsList class="mb-4 w-fit gap-0">
          <TabsIndicator />
          <TabsTrigger value="whitelist">
            <div class="flex items-center gap-2 py-1">
              <div class="i-hugeicons:shield-check h-4 w-4" />
              <Trans key="instances:_trn_server_players_whitelist" />
            </div>
          </TabsTrigger>
          <TabsTrigger value="ops">
            <div class="flex items-center gap-2 py-1">
              <div class="i-hugeicons:crown h-4 w-4" />
              <Trans key="instances:_trn_server_players_operators" />
            </div>
          </TabsTrigger>
          <TabsTrigger value="banned">
            <div class="flex items-center gap-2 py-1">
              <div class="i-hugeicons:cancel-circle h-4 w-4" />
              <Trans key="instances:_trn_server_players_banned" />
            </div>
          </TabsTrigger>
          <TabsTrigger value="banned-ips">
            <div class="flex items-center gap-2 py-1">
              <div class="i-hugeicons:global-reject h-4 w-4" />
              <Trans key="instances:_trn_server_players_banned_ips" />
            </div>
          </TabsTrigger>
        </TabsList>

        <TabsContent value="whitelist">
          <div class="flex flex-col gap-4">
            <div class="flex items-center gap-2">
              <Input
                class="flex-1"
                placeholder={t(
                  "instances:_trn_server_players_username_placeholder"
                )}
                value={newUsername()}
                onInput={(e) => setNewUsername(e.currentTarget.value)}
                onKeyDown={(e: KeyboardEvent) =>
                  e.key === "Enter" && handleAddWhitelist()
                }
              />
              <Button
                size="small"
                onClick={handleAddWhitelist}
                loading={addWhitelistMutation.isPending}
                disabled={!newUsername().trim()}
              >
                <div class="i-hugeicons:add-circle-half-dot h-4 w-4" />
                <Trans key="instances:_trn_server_players_add" />
              </Button>
            </div>
            <PlayerList
              entries={whitelistQuery.data ?? []}
              onRemove={async (uuid) => {
                await removeWhitelistMutation.mutateAsync({
                  serverId: serverId(),
                  uuid
                })
                whitelistQuery.refetch()
              }}
              nameField="name"
              idField="uuid"
            />
          </div>
        </TabsContent>

        <TabsContent value="ops">
          <div class="flex flex-col gap-4">
            <div class="flex items-center gap-2">
              <Input
                class="flex-1"
                placeholder={t(
                  "instances:_trn_server_players_username_placeholder"
                )}
                value={newUsername()}
                onInput={(e) => setNewUsername(e.currentTarget.value)}
                onKeyDown={(e: KeyboardEvent) =>
                  e.key === "Enter" && handleAddOp()
                }
              />
              <Select
                value={opLevel()}
                onChange={(v) => {
                  if (v) setOpLevel(v)
                }}
                options={["1", "2", "3", "4"]}
                disallowEmptySelection
                itemComponent={(itemProps) => (
                  <SelectItem item={itemProps.item}>
                    {opLevelLabel(itemProps.item.rawValue)}
                  </SelectItem>
                )}
              >
                <SelectTrigger class="min-w-50">
                  <SelectValue<string>>
                    {(state) => opLevelLabel(state.selectedOption())}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent />
              </Select>
              <Button
                size="small"
                onClick={handleAddOp}
                loading={addOpMutation.isPending}
                disabled={!newUsername().trim()}
              >
                <div class="i-hugeicons:add-circle-half-dot h-4 w-4" />
                <Trans key="instances:_trn_server_players_add" />
              </Button>
            </div>
            <PlayerList
              entries={(opsQuery.data ?? []).map((e: any) => ({
                ...e,
                extra: opLevelLabel(String(e.level))
              }))}
              onRemove={async (uuid) => {
                await removeOpMutation.mutateAsync({
                  serverId: serverId(),
                  uuid
                })
                opsQuery.refetch()
              }}
              nameField="name"
              idField="uuid"
            />
          </div>
        </TabsContent>

        <TabsContent value="banned">
          <div class="flex flex-col gap-4">
            <div class="flex items-center gap-2">
              <Input
                class="flex-1"
                placeholder={t(
                  "instances:_trn_server_players_username_placeholder"
                )}
                value={newUsername()}
                onInput={(e) => setNewUsername(e.currentTarget.value)}
                onKeyDown={(e: KeyboardEvent) =>
                  e.key === "Enter" && handleBanPlayer()
                }
              />
              <Input
                class="flex-1"
                placeholder={t(
                  "instances:_trn_server_players_reason_placeholder"
                )}
                value={banReason()}
                onInput={(e) => setBanReason(e.currentTarget.value)}
              />
              <Button
                size="small"
                variant="red"
                onClick={handleBanPlayer}
                loading={banPlayerMutation.isPending}
                disabled={!newUsername().trim()}
              >
                <div class="i-hugeicons:cancel-circle h-4 w-4" />
                <Trans key="instances:_trn_server_players_ban" />
              </Button>
            </div>
            <PlayerList
              entries={(bannedQuery.data ?? []).map((e: any) => ({
                ...e,
                extra: e.reason !== "Banned by operator" ? e.reason : undefined
              }))}
              onRemove={async (uuid) => {
                await unbanPlayerMutation.mutateAsync({
                  serverId: serverId(),
                  uuid
                })
                bannedQuery.refetch()
              }}
              nameField="name"
              idField="uuid"
              removeLabel="instances:_trn_server_players_unban"
            />
          </div>
        </TabsContent>

        <TabsContent value="banned-ips">
          <div class="flex flex-col gap-4">
            <div class="flex items-center gap-2">
              <Input
                class="flex-1"
                placeholder={t("instances:_trn_server_players_ip_placeholder")}
                value={banIp()}
                onInput={(e) => setBanIp(e.currentTarget.value)}
                onKeyDown={(e: KeyboardEvent) =>
                  e.key === "Enter" && handleBanIp()
                }
              />
              <Input
                class="flex-1"
                placeholder={t(
                  "instances:_trn_server_players_reason_placeholder"
                )}
                value={banReason()}
                onInput={(e) => setBanReason(e.currentTarget.value)}
              />
              <Button
                size="small"
                variant="red"
                onClick={handleBanIp}
                loading={banIpMutation.isPending}
                disabled={!banIp().trim()}
              >
                <div class="i-hugeicons:cancel-circle h-4 w-4" />
                <Trans key="instances:_trn_server_players_ban" />
              </Button>
            </div>
            <PlayerList
              entries={(bannedIpsQuery.data ?? []).map((e: any) => ({
                name: e.ip,
                uuid: e.ip,
                extra: e.reason
              }))}
              onRemove={async (ip) => {
                await unbanIpMutation.mutateAsync({ serverId: serverId(), ip })
                bannedIpsQuery.refetch()
              }}
              nameField="name"
              idField="uuid"
              removeLabel="instances:_trn_server_players_unban"
            />
          </div>
        </TabsContent>
      </Tabs>
    </div>
  )
}

interface PlayerListEntry {
  name: string
  uuid: string
  extra?: string
}

interface PlayerListProps {
  entries: PlayerListEntry[]
  onRemove: (id: string) => Promise<void>
  nameField: string
  idField: string
  removeLabel?: NamespacedTranslationKey
}

const PlayerList = (props: PlayerListProps) => {
  return (
    <div class="border-darkSlate-600 bg-darkSlate-900 rounded-xl border">
      <Show
        when={props.entries.length > 0}
        fallback={
          <div class="text-lightSlate-700 flex items-center justify-center py-8 text-sm">
            <Trans key="instances:_trn_server_players_no_entries" />
          </div>
        }
      >
        <For each={props.entries}>
          {(entry) => (
            <div class="border-darkSlate-600 flex items-center justify-between border-b px-4 py-3 last:border-b-0">
              <div class="flex items-center gap-3">
                <img
                  src={`https://mc-heads.net/avatar/${entry.name}/24`}
                  alt=""
                  class="h-6 w-6 rounded"
                  loading="lazy"
                />
                <span class="text-lightSlate-200 text-sm">{entry.name}</span>
                <Show when={entry.extra}>
                  <span class="bg-darkSlate-700 text-lightSlate-500 rounded px-1.5 py-0.5 text-xs">
                    {entry.extra}
                  </span>
                </Show>
              </div>
              <Button
                size="small"
                type="transparent"
                onClick={() => props.onRemove(entry.uuid)}
              >
                <div class="i-hugeicons:delete-02 h-4 w-4 text-red-400" />
                <Trans
                  key={
                    props.removeLabel ?? "instances:_trn_server_players_remove"
                  }
                />
              </Button>
            </div>
          )}
        </For>
      </Show>
    </div>
  )
}

export default PlayersTab
