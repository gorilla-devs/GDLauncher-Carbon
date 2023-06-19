import { QueryClient } from "@tanstack/solid-query";
import {
  createClient,
  wsLink,
  createWSClient,
  Unsubscribable,
} from "@rspc/client";
import { createSolidQueryHooks } from "@rspc/solid";
import type { Procedures } from "@gd/core_module";

export const rspc = createSolidQueryHooks<Procedures>();
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false, // default: true
    },
  },
});

export let port: number | null = null;

export default function initRspc(_port: number) {
  port = _port;
  const wsClient = createWSClient({
    url: `ws://localhost:${_port}/rspc/ws`,
  });

  const client = createClient<Procedures>({
    links: [
      wsLink({
        client: wsClient,
      }),
    ],
  });

  const createInvalidateQuery = () => {
    const context = rspc.useContext();
    let subscription: Unsubscribable | null = null;

    wsClient.getConnection()?.addEventListener("open", (_event) => {
      subscription = client.subscription(["invalidateQuery"], {
        onData: (invalidateOperation) => {
          const key = [invalidateOperation!.key];
          if (invalidateOperation.args !== null) {
            key.concat(invalidateOperation.args);
          }
          context.queryClient.invalidateQueries(key);
        },
      });
    });

    wsClient.getConnection()?.addEventListener("close", (_event) => {
      subscription?.unsubscribe();
    });
  };

  return {
    client,
    createInvalidateQuery,
  };
}
