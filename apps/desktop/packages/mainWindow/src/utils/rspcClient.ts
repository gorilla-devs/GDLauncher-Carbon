import { QueryClient } from "@tanstack/solid-query";
import { createClient, wsLink, createWSClient } from "@rspc/client";
import { createSolidQueryHooks } from "@rspc/solid";
import type { Procedures } from "@gd/core_module";

export const rspc = createSolidQueryHooks<Procedures>();
export const queryClient = new QueryClient();

export default function initRspc(port: number) {
  const wsClient = createWSClient({
    url: `ws://localhost:${port}/rspc/ws`,
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
    client.subscription(["invalidateQuery"], {
      onData: (invalidateOperation) => {
        const key = [invalidateOperation!.key];
        if (invalidateOperation.args !== null) {
          key.concat(invalidateOperation.args);
        }
        context.queryClient.invalidateQueries(key);
      },
    });
  };

  return {
    client,
    createInvalidateQuery,
  };
}
