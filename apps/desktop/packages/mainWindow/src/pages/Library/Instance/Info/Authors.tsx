import { Show, createEffect, createSignal, createMemo } from "solid-js"
import {
  CFFEModAuthor,
  FEModResponse,
  MRFEProject,
  MRFETeamMember,
  MRFETeamResponse
} from "@gd/core_module/bindings"
import { rspc } from "@/utils/rspcClient"
import AuthorAvatars, { Author } from "@/components/AuthorAvatars"
import { AuthorsSkeleton } from "@gd/ui"

interface Props {
  modpackDetails: FEModResponse | MRFEProject | undefined
  isCurseforge: boolean
  isModrinth: boolean
}

const Authors = (props: Props) => {
  const [authors, setAuthors] = createSignal<MRFETeamResponse>([])
  const [isLoading, setIsLoading] = createSignal(false)
  const rspcContext = rspc.useContext()

  createEffect(async () => {
    if (
      props.modpackDetails &&
      props.isModrinth &&
      (props.modpackDetails as MRFEProject)?.team
    ) {
      setIsLoading(true)
      try {
        const modrinthAuthorsQuery = await rspcContext.client.query([
          "modplatforms.modrinth.getTeam",
          (props.modpackDetails as MRFEProject)?.team
        ])

        if (modrinthAuthorsQuery) setAuthors(modrinthAuthorsQuery)
      } finally {
        setIsLoading(false)
      }
    }
  })

  const getAuthors = () => {
    if (props.isCurseforge && props.modpackDetails) {
      const modpack = props.modpackDetails as FEModResponse
      return modpack.data?.authors
    } else if (props.isModrinth) return authors()

    return []
  }

  const normalizedAuthors = createMemo((): Author[] => {
    const rawAuthors = getAuthors()
    if (!rawAuthors) return []

    if (props.isCurseforge) {
      return (rawAuthors as CFFEModAuthor[]).map((author) => ({
        name: author.name,
        avatarUrl: author.avatarUrl,
        url: author.url,
        id: author.id,
        platform: "curseforge" as const
      }))
    } else if (props.isModrinth) {
      return (rawAuthors as MRFETeamMember[]).map((member) => ({
        name: member.user.name || member.user.username,
        avatarUrl: member.user.avatar_url,
        role: member.role,
        id: member.user.id,
        platform: "modrinth" as const
      }))
    }

    return []
  })

  return (
    <Show
      when={!isLoading() && normalizedAuthors().length > 0}
      fallback={
        <Show when={isLoading()}>
          <AuthorsSkeleton count={3} size="md" />
        </Show>
      }
    >
      <AuthorAvatars authors={normalizedAuthors()} maxDisplay={4} />
    </Show>
  )
}

export default Authors
