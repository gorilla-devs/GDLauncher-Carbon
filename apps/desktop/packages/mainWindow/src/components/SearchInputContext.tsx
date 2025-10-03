import { createContext, useContext } from "solid-js"
import { getSearchResults } from "@/utils/platformSearch"

export const SearchInputContext = createContext<
  ReturnType<typeof getSearchResults> | undefined
>()

export default function useSearchContext() {
  return useContext(SearchInputContext)
}
