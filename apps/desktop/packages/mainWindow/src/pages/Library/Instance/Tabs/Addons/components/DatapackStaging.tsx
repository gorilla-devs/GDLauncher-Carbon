import { Show, createSignal, createEffect, on } from "solid-js"
import type { Mod } from "@gd/core_module/bindings"
import { Trans, useTransContext } from "@gd/i18n"
import { rspc } from "@/utils/rspcClient"
import {
  Button,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  toast
} from "@gd/ui"

export function DatapackStaging(props: {
  instanceId: number
  packs: Mod[]
  locked: boolean
}) {
  const [t] = useTransContext()
  const [packId, setPackId] = createSignal("")
  const [worldIdentity, setWorldIdentity] = createSignal("")
  createEffect(
    on(
      () => props.instanceId,
      () => {
        setPackId("")
        setWorldIdentity("")
      }
    )
  )
  const worlds = rspc.createQuery(() => ({
    queryKey: ["instance.getDatapackWorlds", props.instanceId],
    enabled: props.packs.length > 0 && !props.locked
  }))
  const copy = rspc.createMutation(() => ({
    mutationKey: ["instance.copyDatapack"]
  }))
  const zipPacks = () =>
    props.packs.filter((pack) => pack.filename.toLowerCase().endsWith(".zip"))
  const packName = (id: string | null | undefined) =>
    props.packs.find((pack) => pack.id === id)?.filename
  const worldName = (identity: string | null | undefined) =>
    worlds.data?.find((world) => world.identity === identity)?.name
  const selectedWorld = () =>
    worlds.data?.find((world) => world.identity === worldIdentity())
  const publish = async () => {
    const world = selectedWorld()
    const pack = props.packs.find((pack) => pack.id === packId())
    if (!world || !pack) return
    const request = {
      instance_id: props.instanceId,
      mod_id: pack.id,
      world: { ...world }
    }
    const title = pack.filename
    try {
      const destination = await copy.mutateAsync(request)
      toast.success(
        t("content:_trn_datapack_staging.added", {
          name: title,
          world: destination,
          interpolation: { escapeValue: false }
        })
      )
      if (props.instanceId === request.instance_id) await worlds.refetch()
    } catch {
      /* MutationCache displays the actual error; selection remains retryable. */
    }
  }
  return (
    <Show when={props.packs.length > 0}>
      <section class="mx-6 my-3 rounded border border-darkSlate-600 p-4">
        <p class="font-medium">
          <Trans
            key="content:_trn_datapack_staging.title"
            options={{ count: props.packs.length }}
          />
        </p>
        <p class="text-sm text-lightSlate-400">
          <Trans key="content:_trn_datapack_staging.description" />
        </p>
        <div class="mt-3 flex flex-wrap items-center gap-3">
          <Select
            value={packId() || null}
            onChange={(id) => setPackId(id ?? "")}
            options={zipPacks().map((pack) => pack.id)}
            placeholder={t("content:_trn_datapack_staging.choose_pack")}
            itemComponent={(itemProps) => (
              <SelectItem item={itemProps.item}>
                {packName(itemProps.item.rawValue)}
              </SelectItem>
            )}
          >
            <SelectTrigger
              class="w-56"
              aria-label={t("content:_trn_datapack_staging.pack_label")}
            >
              <SelectValue<string>>
                {(state) =>
                  packName(state.selectedOption()) ??
                  t("content:_trn_datapack_staging.choose_pack")
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
          <Select
            value={worldIdentity() || null}
            onChange={(identity) => setWorldIdentity(identity ?? "")}
            options={(worlds.data ?? []).map((world) => world.identity)}
            placeholder={t("content:_trn_datapack_staging.choose_world")}
            itemComponent={(itemProps) => (
              <SelectItem item={itemProps.item}>
                {worldName(itemProps.item.rawValue)}
              </SelectItem>
            )}
          >
            <SelectTrigger
              class="w-56"
              aria-label={t("content:_trn_datapack_staging.world_label")}
            >
              <SelectValue<string>>
                {(state) =>
                  worldName(state.selectedOption()) ??
                  t("content:_trn_datapack_staging.choose_world")
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent />
          </Select>
          <Button
            size="small"
            disabled={
              props.locked ||
              copy.isPending ||
              !selectedWorld() ||
              !props.packs.some((pack) => pack.id === packId())
            }
            onClick={publish}
          >
            <Trans key="content:_trn_datapack_staging.add_to_world" />
          </Button>
          <Button
            size="small"
            type="secondary"
            onClick={() => worlds.refetch()}
            disabled={props.locked}
          >
            <Trans key="content:_trn_datapack_staging.refresh_worlds" />
          </Button>
        </div>
        <Show when={!worlds.isLoading && worlds.data?.length === 0}>
          <p class="mt-2 text-sm">
            <Trans key="content:_trn_datapack_staging.no_worlds" />
          </p>
        </Show>
      </section>
    </Show>
  )
}
