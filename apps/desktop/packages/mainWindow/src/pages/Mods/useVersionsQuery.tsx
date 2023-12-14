import { createStore } from "solid-js/store";

export const versionsDefaultQuery = {
  gameVersion: null,
  index: 0,
  modLoaderType: null,
  pageSize: 20
};

const useVersionsQuery = (
  initialValue?: typeof versionsDefaultQuery
): [
  typeof versionsDefaultQuery,
  (_newValue: Partial<typeof versionsDefaultQuery>) => void
] => {
  const [query, setQuery] = createStore<typeof versionsDefaultQuery>({
    ...versionsDefaultQuery,
    ...initialValue
  });

  const setQueryParams = (newValue: Partial<typeof versionsDefaultQuery>) => {
    const indexValue = newValue.index ?? 0;

    setQuery((prev) => ({
      ...prev,
      ...newValue,
      index: indexValue
    }));
  };

  return [query, setQueryParams];
};

export default useVersionsQuery;
