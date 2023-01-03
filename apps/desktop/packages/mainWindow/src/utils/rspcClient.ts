import { QueryClient } from "@tanstack/solid-query";
import { WebsocketTransport, createClient, FetchTransport } from "@rspc/client";
import { createSolidQueryHooks } from "@rspc/solid";

import type { Procedures } from "@gd/carbon_core"; // These were the bindings exported from your Rust code!

export const client = createClient<Procedures>({
  transport: new WebsocketTransport("ws://localhost:4000/rspc/ws"),
  // transport: new FetchTransport("http://localhost:4000/rspc"),
});

export const queryClient = new QueryClient();
export const rspc = createSolidQueryHooks<Procedures>();

client.addSubscription(["pings"], {
  onData: (e) => console.log(e),
});
