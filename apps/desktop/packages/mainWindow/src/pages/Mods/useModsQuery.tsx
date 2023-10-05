import { FEUnifiedSearchParameters } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";

const useModsQuery = (
  initialValue?: FEUnifiedSearchParameters
): [
  FEUnifiedSearchParameters,
  (_newValue: Partial<FEUnifiedSearchParameters>) => void
] => {
  const [query, setQuery] = createStore<FEUnifiedSearchParameters>({
    searchQuery: "",
    categories: null,
    gameVersions: null,
    modloaders: null,
    projectType: "mod",
    sortIndex: { curseForge: "featured" },
    sortOrder: "descending",
    index: 0,
    pageSize: 40,
    searchApi: "curseforge",
    ...initialValue
  });

  const setQueryParams = (newValue: Partial<FEUnifiedSearchParameters>) => {
    const indexValue = newValue.index ?? 0;

    setQuery((prev) => ({
      ...prev,
      ...newValue,
      index: indexValue
    }));
  };

  return [query, setQueryParams];
};

export default useModsQuery;
