import {
  FEModSearchParameters,
  FEModSearchParametersQuery,
} from "@gd/core_module/bindings";
import { Accessor, createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";

const useModpacksQuery = (
  initialValue?: FEModSearchParametersQuery
): [
  FEModSearchParameters,
  (_newValue: Partial<FEModSearchParametersQuery>) => void,
  (_isFirstIncrement?: boolean) => void,
  Accessor<boolean>
] => {
  const [replaceList, setReplaceList] = createSignal(false);
  const [query, setQuery] = createStore<FEModSearchParameters>({
    query: initialValue || {
      categoryId: 0,
      classId: "modpacks",
      gameId: 432,
      gameVersion: "",
      page: 1,
      modLoaderType: "any",
      sortField: "featured",
      sortOrder: "descending",
      pageSize: 20,
      slug: "",
      searchFilter: "",
      gameVersionTypeId: null,
      authorId: null,
      index: 0,
    },
  });

  const setQueryParams = (newValue: Partial<FEModSearchParametersQuery>) => {
    const indexValue = newValue.index ?? 0;

    if ("index" in newValue) {
      setReplaceList(false);
    } else {
      setReplaceList(true);
    }

    setQuery("query", (prev) => ({
      ...prev,
      ...newValue,
      index: indexValue,
    }));
  };

  const incrementIndex = (isFirstIncrement?: boolean) => {
    const pageSize = query.query.pageSize || 40;

    if (!isFirstIncrement) setReplaceList(false);

    setQuery(
      "query",
      produce((prev) => (prev.index = (prev.index as number) + pageSize))
    );
  };

  return [query, setQueryParams, incrementIndex, replaceList];
};

export default useModpacksQuery;
