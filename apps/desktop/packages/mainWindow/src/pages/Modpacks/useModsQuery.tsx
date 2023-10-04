import { FEUnifiedSearchParameters } from "@gd/core_module/bindings";
import { createStore } from "solid-js/store";

const useModpacksQuery = (
  initialValue?: FEUnifiedSearchParameters
): [
  FEUnifiedSearchParameters,
  (_newValue: Partial<FEUnifiedSearchParameters>) => void
] => {
  const [query, setQuery] = createStore<FEUnifiedSearchParameters>({
    ...initialValue,
    searchQuery: "",
    categories: null,
    gameVersions: null,
    modloaders: null,
    projectType: "modPack",
    sortIndex: { curseForge: "featured" },
    sortOrder: "descending",
    index: 0,
    pageSize: 40,
    searchApi: "curseforge"
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

export default useModpacksQuery;
