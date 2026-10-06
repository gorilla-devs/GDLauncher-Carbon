import {
  loadLanguageFiles,
  useTransContext,
  normalizeLanguageId,
  getLanguageLocaleCode
} from "@gd/i18n"
import { rspc } from "./rspcClient"

let latestSelection = 0
let publication: Promise<void> = Promise.resolve()

/**
 * Change the language of the app
 * To be able to call this function, you need to use getOwner and runWithOwner because context is not available outside of computations
 * Call
 * ```
 *  const owner = getOwner();
 * ```
 *
 * and then call the function as such
 * ```
 *  runWithOwner(owner, () => {
 *    changeLanguage(value);
 *  });
 *  ```
 */
export default async function changeLanguage(language: string) {
  const lang = normalizeLanguageId(language)
  const localeCode = getLanguageLocaleCode(lang)
  const selection = ++latestSelection
  const [_, { changeLanguage, addResources }] = useTransContext()

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"]
  }))

  const resources = await loadLanguageFiles(lang)
  if (selection !== latestSelection) return
  const publish = publication.then(async () => {
    if (selection !== latestSelection) return
    for (const ns in resources) {
      addResources(localeCode, ns, resources[ns])
    }
    // Once dispatched, an older write must commit before the next write can
    // start. Resource ordering alone cannot prevent reversed DB commits.
    await settingsMutation.mutateAsync({ language: { Set: lang } })
    if (selection === latestSelection) changeLanguage(localeCode)
  })
  publication = publish.catch(() => {})
  await publish
}
