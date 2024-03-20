import { loadLanguageFiles, useTransContext } from "@gd/i18n";
import { queryClient, rspc } from "./rspcClient";

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
export default async function changeLanguage(lang: string) {
  const [_, { changeLanguage, addResources }] = useTransContext();

  const settingsMutation = rspc.createMutation(() => ({
    mutationKey: ["settings.setSettings"],
    onMutate: (newSettings) => {
      queryClient.setQueryData(["settings.getSettings"], newSettings);
    }
  }));

  const resources = await loadLanguageFiles(lang);
  for (const ns in resources) {
    addResources(lang, ns, resources[ns]);
  }

  settingsMutation.mutate({
    language: {
      Set: lang
    }
  });

  changeLanguage(lang);
}
