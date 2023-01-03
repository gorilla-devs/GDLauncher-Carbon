import { QueryClient } from "@tanstack/solid-query";
import {
  WebsocketTransport,
  createClient,
  FetchTransport,
  wsLink,
  createWSClient,
} from "@rspc/client";
import { createSolidQueryHooks } from "@rspc/solid";

import type { Procedures } from "@gd/carbon_core"; // These were the bindings exported from your Rust code!

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

client.subscription(["pings"], {
  onData: (e) => console.log(e),
});
