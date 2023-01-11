import { QueryClient } from "@tanstack/solid-query";
import { createClient, wsLink, createWSClient } from "@rspc/client";
import { createSolidQueryHooks } from "@rspc/solid";

import type { Procedures } from "@gd/native_interface"; // These were the bindings exported from your Rust code!

const wsClient = createWSClient({
  url: "ws://localhost:4000/rspc/ws",
});

export const client = createClient<Procedures>({
  links: [
    wsLink({
      client: wsClient,
    }),
  ],
});

export const queryClient = new QueryClient();
export const rspc = createSolidQueryHooks<Procedures>();

export function createInvalidateQuery() {
  const context = rspc.useContext();
  client.subscription(["invalidateQuery"], {
    onData: (invalidateOperation: any) => {
      const key = [invalidateOperation!.key];
      if (invalidateOperation.arg !== null) {
        key.concat(invalidateOperation.arg);
      }
      context.queryClient.invalidateQueries(key);
    },
  });
}
