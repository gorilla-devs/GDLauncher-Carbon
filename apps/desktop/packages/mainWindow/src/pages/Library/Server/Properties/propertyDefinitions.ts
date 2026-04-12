export interface PropertyDefinition {
  key: string
  label: string
  type: "string" | "number" | "boolean" | "enum"
  defaultValue: string
  enumValues?: string[]
  enumLabels?: string[]
  min?: number
  max?: number
  description?: string
  /** Detailed explanation shown in info tooltip */
  info?: string
  /** Minimum MC version this property exists in (e.g. "1.18") */
  minVersion?: string
  /** MC version this property was removed in (e.g. "1.13") */
  removedVersion?: string
}

export interface PropertyGroup {
  id: string
  label: string
  icon: string
  properties: PropertyDefinition[]
}

/**
 * Compare two MC version strings. Returns -1, 0, or 1.
 * Handles formats like "1.20.1", "1.8", "1.21.2".
 */
export function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map(Number)
  const pb = b.split(".").map(Number)
  const len = Math.max(pa.length, pb.length)
  for (let i = 0; i < len; i++) {
    const na = pa[i] ?? 0
    const nb = pb[i] ?? 0
    if (na < nb) return -1
    if (na > nb) return 1
  }
  return 0
}

/** Check if a property is available for a given MC version */
export function isAvailableForVersion(
  prop: PropertyDefinition,
  gameVersion: string
): boolean {
  if (prop.minVersion && compareVersions(gameVersion, prop.minVersion) < 0) {
    return false
  }
  if (
    prop.removedVersion &&
    compareVersions(gameVersion, prop.removedVersion) >= 0
  ) {
    return false
  }
  return true
}

export const propertyGroups: PropertyGroup[] = [
  {
    id: "gameplay",
    label: "Gameplay",
    icon: "i-hugeicons:joystick-03",
    properties: [
      {
        key: "difficulty",
        label: "Difficulty",
        type: "enum",
        defaultValue: "easy",
        enumValues: ["peaceful", "easy", "normal", "hard"],
        description: "Server difficulty level",
        info: "Sets the world difficulty. Peaceful suppresses hostile mob spawning entirely. Hard enables the most challenging gameplay including starvation damage."
      },
      {
        key: "gamemode",
        label: "Default Gamemode",
        type: "enum",
        defaultValue: "survival",
        enumValues: ["survival", "creative", "adventure", "spectator"],
        description: "Gamemode assigned to new players",
        info: "The game mode players are assigned when they first join the server. Spectator mode was added in 1.8."
      },
      {
        key: "force-gamemode",
        label: "Force Gamemode",
        type: "boolean",
        defaultValue: "false",
        description: "Force players to join in default gamemode",
        info: "When enabled, every player is reset to the default gamemode each time they connect, overriding their stored game mode.",
        minVersion: "1.2"
      },
      {
        key: "hardcore",
        label: "Hardcore",
        type: "boolean",
        defaultValue: "false",
        info: "Enables hardcore mode: difficulty is locked to Hard and players are banned upon death rather than respawning. Changing this after world creation has no retroactive effect."
      },
      {
        key: "pvp",
        label: "PvP",
        type: "boolean",
        defaultValue: "true",
        info: "Controls whether players can damage one another. When disabled, players can still harm mobs and themselves; only direct player-versus-player damage is blocked."
      },
      {
        key: "spawn-protection",
        label: "Spawn Protection",
        type: "number",
        defaultValue: "16",
        min: 0,
        max: 1000,
        description: "Radius in blocks (0 = disabled)",
        info: "Radius in blocks around the world spawn point where only operators can build or break blocks. The protected area is a square of side 2N+1 centered on spawn. Set to 0 to disable."
      },
      {
        key: "enable-command-block",
        label: "Command Blocks",
        type: "boolean",
        defaultValue: "false",
        info: "Whether command blocks can be used on the server. Even if enabled, players still need operator permissions to place or interact with them.",
        minVersion: "1.4.2"
      },
      {
        key: "spawn-monsters",
        label: "Spawn Monsters",
        type: "boolean",
        defaultValue: "true",
        info: "Whether hostile mobs (zombies, skeletons, creepers, etc.) are allowed to spawn naturally. Overridden to false when difficulty is Peaceful."
      },
      {
        key: "spawn-animals",
        label: "Spawn Animals",
        type: "boolean",
        defaultValue: "true",
        info: "Whether passive and neutral animals (cows, pigs, wolves, etc.) can spawn naturally. Does not affect animals spawned through breeding or spawn eggs."
      },
      {
        key: "spawn-npcs",
        label: "Spawn NPCs",
        type: "boolean",
        defaultValue: "true",
        info: "Whether villagers can spawn in villages and through breeding. Also controls wandering traders in newer versions.",
        minVersion: "1.3"
      },
      {
        key: "allow-flight",
        label: "Allow Flight",
        type: "boolean",
        defaultValue: "false",
        info: "When disabled, the server kicks players detected as flying in Survival or Adventure mode (anti-cheat). Enable this when using mods or plugins that grant legitimate flight."
      },
      {
        key: "allow-nether",
        label: "Allow Nether",
        type: "boolean",
        defaultValue: "true",
        info: "Whether players can travel through Nether portals. When disabled, portal blocks still exist but cannot be activated."
      },
      {
        key: "player-idle-timeout",
        label: "Idle Timeout",
        type: "number",
        defaultValue: "0",
        min: 0,
        max: 1440,
        description: "Minutes before kick (0 = disabled)",
        info: "Number of minutes of inactivity before a player is kicked. Inactivity is measured as no movement, no chat, and no inventory interaction. Set to 0 to disable.",
        minVersion: "1.7.10"
      },
      {
        key: "op-permission-level",
        label: "OP Permission Level",
        type: "enum",
        defaultValue: "4",
        enumValues: ["1", "2", "3", "4"],
        enumLabels: [
          "1 - Bypass spawn protection",
          "2 - Cheats & command blocks",
          "3 - Kick, ban & op players",
          "4 - Full control (stop server)"
        ],
        info: "Determines what operators can do. Level 1: bypass spawn protection. Level 2: use cheats and command blocks. Level 3: kick, ban, and op/deop players. Level 4: use /stop and have full server control.",
        minVersion: "1.7.10"
      },
      {
        key: "function-permission-level",
        label: "Function Permission Level",
        type: "enum",
        defaultValue: "2",
        enumValues: ["1", "2", "3", "4"],
        enumLabels: [
          "1 - Bypass spawn protection",
          "2 - Cheats & command blocks",
          "3 - Kick, ban & op players",
          "4 - Full control (stop server)"
        ],
        info: "The permission level that datapack functions and the /function command run with. Higher values allow functions to execute more privileged commands.",
        minVersion: "1.12"
      }
    ]
  },
  {
    id: "world",
    label: "World",
    icon: "i-hugeicons:earth",
    properties: [
      {
        key: "level-name",
        label: "World Name",
        type: "string",
        defaultValue: "world",
        info: "Name of the directory inside the server folder that contains the world save. Changing this effectively switches worlds. If the directory does not exist, a new world is generated."
      },
      {
        key: "level-seed",
        label: "World Seed",
        type: "string",
        defaultValue: "",
        info: "Seed used during world generation. Accepts any integer or arbitrary string (which is hashed internally). An empty string causes a random seed. Has no effect once the world has been created."
      },
      {
        key: "level-type",
        label: "World Type",
        type: "enum",
        defaultValue: "minecraft\\:normal",
        enumValues: [
          "minecraft\\:normal",
          "minecraft\\:flat",
          "minecraft\\:large_biomes",
          "minecraft\\:amplified",
          "minecraft\\:single_biome_surface"
        ],
        enumLabels: ["Normal", "Superflat", "Large Biomes", "Amplified", "Single Biome"],
        info: "World generator preset. Normal generates standard terrain. Superflat creates a flat world. Large Biomes increases biome size by 16x. Amplified exaggerates terrain height. Single Biome generates the entire world as one biome."
      },
      {
        key: "generator-settings",
        label: "Generator Settings",
        type: "string",
        defaultValue: "{}",
        info: "A JSON string providing additional parameters for the world generator. Primarily used with Superflat to define layers, biome, and features. For other world types this is typically left empty.",
        minVersion: "1.1"
      },
      {
        key: "generate-structures",
        label: "Generate Structures",
        type: "boolean",
        defaultValue: "true",
        info: "Whether naturally generated structures (villages, dungeons, strongholds, etc.) are created during world generation. Has no effect on already-generated chunks."
      },
      {
        key: "max-world-size",
        label: "Max World Size",
        type: "number",
        defaultValue: "29999984",
        min: 1,
        max: 29999984,
        description: "Maximum world border radius in blocks",
        info: "The maximum radius in blocks from world center that the world border can extend to. Players cannot expand the border beyond this value via commands.",
        minVersion: "1.7.10"
      },
      {
        key: "initial-enabled-packs",
        label: "Initial Enabled Packs",
        type: "string",
        defaultValue: "vanilla",
        description: "Comma-separated list",
        info: "Comma-separated list of datapacks to enable when creating a new world. Only takes effect during initial world creation; use /datapack commands for existing worlds.",
        minVersion: "1.19.3"
      },
      {
        key: "initial-disabled-packs",
        label: "Initial Disabled Packs",
        type: "string",
        defaultValue: "",
        description: "Comma-separated list",
        info: "Comma-separated list of datapacks to disable when a new world is created. Useful for disabling experimental datapacks shipped in some versions. Only applies at world creation.",
        minVersion: "1.19.3"
      },
      {
        key: "region-file-compression",
        label: "Region File Compression",
        type: "enum",
        defaultValue: "deflate",
        enumValues: ["deflate", "lz4", "none"],
        enumLabels: ["Deflate (default, most compatible)", "LZ4 (faster, slightly larger)", "None (no compression)"],
        info: "Algorithm used to compress region files. Deflate is the standard format with maximum compatibility. LZ4 is ~2-3x faster but produces slightly larger files. Changing this does not recompress existing chunks.",
        minVersion: "1.20.5"
      }
    ]
  },
  {
    id: "network",
    label: "Network",
    icon: "i-hugeicons:wifi-01",
    properties: [
      {
        key: "server-port",
        label: "Server Port",
        type: "number",
        defaultValue: "25565",
        min: 1,
        max: 65535,
        info: "TCP port the server listens on. The standard Minecraft port is 25565. Change this if running multiple servers or if the port is already in use."
      },
      {
        key: "server-ip",
        label: "Server IP",
        type: "string",
        defaultValue: "",
        description: "Leave empty to bind all interfaces",
        info: "IP address the server binds to. An empty string causes the server to listen on all available network interfaces (0.0.0.0). Specify an IP to restrict binding to a single interface."
      },
      {
        key: "motd",
        label: "MOTD",
        type: "string",
        defaultValue: "A Minecraft Server",
        info: "The Message of the Day displayed in the server list. Supports formatting codes using the section sign (§). Can be up to 59 characters."
      },
      {
        key: "max-players",
        label: "Max Players",
        type: "number",
        defaultValue: "20",
        min: 1,
        max: 1000,
        info: "Maximum number of players allowed simultaneously. Additional players receive a 'Server is full' message. Operators can bypass this limit."
      },
      {
        key: "online-mode",
        label: "Online Mode",
        type: "boolean",
        defaultValue: "true",
        description: "Require Mojang authentication",
        info: "When enabled, the server authenticates players with Mojang's session servers, ensuring they own a valid copy. Disable only behind a proxy like Velocity or BungeeCord that handles auth upstream."
      },
      {
        key: "prevent-proxy-connections",
        label: "Prevent Proxy Connections",
        type: "boolean",
        defaultValue: "false",
        info: "When enabled, the server checks the player's IP against Mojang's records, blocking VPN/proxy connections. Requires online-mode to be enabled. Can inadvertently block legitimate users on shared NAT.",
        minVersion: "1.9"
      },
      {
        key: "network-compression-threshold",
        label: "Compression Threshold",
        type: "number",
        defaultValue: "256",
        min: -1,
        max: 65535,
        description: "Bytes (-1 = disabled)",
        info: "Packet size in bytes above which network traffic is compressed. -1 disables compression (increases bandwidth). 0 compresses all packets. Values between 64 and 512 are typical.",
        minVersion: "1.8"
      },
      {
        key: "rate-limit",
        label: "Rate Limit",
        type: "number",
        defaultValue: "0",
        min: 0,
        max: 1000,
        description: "Max packets/sec (0 = disabled)",
        info: "Maximum number of packets a client may send per second before being kicked. A value of 0 disables the rate limit. This is a basic anti-flood measure.",
        minVersion: "1.12"
      },
      {
        key: "enable-status",
        label: "Enable Status",
        type: "boolean",
        defaultValue: "true",
        info: "Whether the server responds to status pings used to populate the multiplayer server list (player count, MOTD, favicon). Disabling makes the server appear offline but players can still connect directly.",
        minVersion: "1.7"
      },
      {
        key: "hide-online-players",
        label: "Hide Online Players",
        type: "boolean",
        defaultValue: "false",
        info: "When enabled, the server reports 0 players online in the server list and hides player names on hover. The real player count is still enforced for max-players.",
        minVersion: "1.18"
      },
      {
        key: "enable-rcon",
        label: "Enable RCON",
        type: "boolean",
        defaultValue: "false",
        info: "Enables the RCON (Remote Console) protocol for remote command execution via TCP. RCON traffic is unencrypted; use only on trusted networks or through an SSH tunnel."
      },
      {
        key: "rcon.port",
        label: "RCON Port",
        type: "number",
        defaultValue: "25575",
        min: 1,
        max: 65535,
        info: "TCP port the RCON server listens on. Must be different from the main server port. Only active when RCON is enabled."
      },
      {
        key: "rcon.password",
        label: "RCON Password",
        type: "string",
        defaultValue: "",
        info: "Password required for RCON authentication. Must be non-empty for RCON to function. Treat this as a sensitive credential."
      },
      {
        key: "enable-query",
        label: "Enable Query",
        type: "boolean",
        defaultValue: "false",
        info: "Enables the GameSpy4 query protocol (UDP), allowing external tools and server listing services to fetch server information without a full connection."
      },
      {
        key: "query.port",
        label: "Query Port",
        type: "number",
        defaultValue: "25565",
        min: 1,
        max: 65535,
        info: "UDP port the query listener uses. Defaults to the main server port but can be set independently. Only active when Query is enabled."
      },
      {
        key: "accepts-transfers",
        label: "Accept Transfers",
        type: "boolean",
        defaultValue: "false",
        info: "Whether the server accepts incoming player transfers from other servers via the /transfer command introduced in 1.20.5. Enable this if your network uses server-to-server transfers.",
        minVersion: "1.20.5"
      }
    ]
  },
  {
    id: "performance",
    label: "Performance",
    icon: "i-hugeicons:dashboard-speed-02",
    properties: [
      {
        key: "view-distance",
        label: "View Distance",
        type: "number",
        defaultValue: "10",
        min: 2,
        max: 32,
        description: "Chunk radius sent to players",
        info: "Radius of chunks sent to each player. Higher values mean more of the world is visible but increase memory, CPU, and network load. Mojang recommends 10 for most servers."
      },
      {
        key: "simulation-distance",
        label: "Simulation Distance",
        type: "number",
        defaultValue: "10",
        min: 2,
        max: 32,
        description: "Chunk radius for active game logic",
        info: "Radius of chunks where game logic (mob spawning, crop growth, redstone, etc.) is actively simulated. Decoupled from view-distance in 1.18; you can render more than you simulate. Lower values significantly reduce server load.",
        minVersion: "1.18"
      },
      {
        key: "entity-broadcast-range-percentage",
        label: "Entity Broadcast Range",
        type: "number",
        defaultValue: "100",
        min: 10,
        max: 1000,
        description: "Percentage of view distance",
        info: "Percentage of the client's view distance at which the server sends entity position updates. Lowering this (e.g., to 50) reduces entity packets at the cost of entities appearing to pop in at shorter distances.",
        minVersion: "1.9"
      },
      {
        key: "max-tick-time",
        label: "Max Tick Time",
        type: "number",
        defaultValue: "60000",
        min: -1,
        max: 600000,
        description: "Milliseconds (-1 = disable watchdog)",
        info: "Maximum time in milliseconds a single tick can run before the watchdog crashes the server. Set to -1 to disable. Increase this on heavily modded servers where long ticks are expected.",
        minVersion: "1.7.10"
      },
      {
        key: "max-chained-neighbor-updates",
        label: "Max Chained Neighbor Updates",
        type: "number",
        defaultValue: "1000000",
        min: -1,
        max: 2147483647,
        description: "-1 = unlimited",
        info: "Limits consecutive block update chains to prevent lag machines and infinite redstone loops. Set to -1 to disable the limit (not recommended on public servers).",
        minVersion: "1.19"
      },
      {
        key: "sync-chunk-writes",
        label: "Sync Chunk Writes",
        type: "boolean",
        defaultValue: "true",
        info: "When enabled, chunk data is written to disk synchronously, preventing data loss on crashes. Disabling allows async writes for performance but risks chunk corruption on unclean shutdowns.",
        minVersion: "1.14"
      },
      {
        key: "use-native-transport",
        label: "Use Native Transport",
        type: "boolean",
        defaultValue: "true",
        info: "On Linux, uses Epoll-based native networking for lower latency and higher throughput. Has no effect on Windows or macOS. Should generally be left enabled on Linux servers.",
        minVersion: "1.12"
      },
      {
        key: "pause-when-empty-seconds",
        label: "Pause When Empty",
        type: "number",
        defaultValue: "60",
        min: 0,
        max: 86400,
        description: "Seconds after last player leaves (0 = disabled)",
        info: "Seconds to wait after the last player disconnects before pausing world simulation. The server freezes ticks while empty, conserving resources. Set to 0 to keep the server running at all times.",
        minVersion: "1.21.2"
      }
    ]
  },
  {
    id: "security",
    label: "Security",
    icon: "i-hugeicons:shield-check",
    properties: [
      {
        key: "white-list",
        label: "Whitelist",
        type: "boolean",
        defaultValue: "false",
        info: "When enabled, only players listed in whitelist.json can join. Non-whitelisted players receive a rejection message. Operators are always allowed regardless of whitelist status."
      },
      {
        key: "enforce-whitelist",
        label: "Enforce Whitelist",
        type: "boolean",
        defaultValue: "false",
        description: "Kick non-whitelisted players on reload",
        info: "When enabled, running /whitelist reload also kicks any currently connected players no longer on the whitelist. Without this, already-connected players are unaffected by whitelist updates.",
        minVersion: "1.7.10"
      },
      {
        key: "enforce-secure-profile",
        label: "Enforce Secure Profile",
        type: "boolean",
        defaultValue: "true",
        info: "When enabled, players must have a cryptographically signed profile from the official Minecraft client. Players without a signed profile are rejected. Disable when running behind a proxy or for offline-mode support.",
        minVersion: "1.19"
      },
      {
        key: "log-ips",
        label: "Log Player IPs",
        type: "boolean",
        defaultValue: "true",
        info: "Controls whether player IP addresses are written to server logs. Disabling replaces IPs with a placeholder for GDPR/privacy compliance. Does not affect Query or RCON traffic.",
        minVersion: "1.19.4"
      },
      {
        key: "broadcast-rcon-to-ops",
        label: "Broadcast RCON to Ops",
        type: "boolean",
        defaultValue: "true",
        info: "When enabled, commands executed via RCON are broadcast to online operators in-game, providing visibility into what automated scripts are doing.",
        minVersion: "1.9"
      },
      {
        key: "broadcast-console-to-ops",
        label: "Broadcast Console to Ops",
        type: "boolean",
        defaultValue: "true",
        info: "When enabled, commands typed in the server console are broadcast to online operators in-game so they can see what administrators are doing.",
        minVersion: "1.9"
      },
      {
        key: "enable-jmx-monitoring",
        label: "JMX Monitoring",
        type: "boolean",
        defaultValue: "false",
        info: "Exposes server performance metrics (TPS, memory, etc.) via Java Management Extensions. Consumable by monitoring tools such as JConsole, VisualVM, or Prometheus JMX exporter.",
        minVersion: "1.16"
      },
      {
        key: "resource-pack",
        label: "Resource Pack URL",
        type: "string",
        defaultValue: "",
        info: "URL of a resource pack (.zip) that players are prompted to download on join. Must be a direct download link. Set resource-pack-sha1 alongside this for integrity verification.",
        minVersion: "1.4.2"
      },
      {
        key: "resource-pack-sha1",
        label: "Resource Pack SHA-1",
        type: "string",
        defaultValue: "",
        info: "SHA-1 hash of the resource pack file for integrity verification and caching. If the hash matches a cached version, the client skips the download. Should be the lowercase hex digest.",
        minVersion: "1.4.2"
      },
      {
        key: "resource-pack-prompt",
        label: "Resource Pack Prompt",
        type: "string",
        defaultValue: "",
        info: "Custom text displayed to players in the resource pack download confirmation dialog. Useful for explaining what the pack contains.",
        minVersion: "1.17"
      },
      {
        key: "require-resource-pack",
        label: "Require Resource Pack",
        type: "boolean",
        defaultValue: "false",
        info: "When enabled, players who decline the resource pack download are disconnected from the server. Only meaningful when a resource-pack URL is set.",
        minVersion: "1.17"
      },
      {
        key: "text-filtering-config",
        label: "Text Filtering Config",
        type: "string",
        defaultValue: "",
        info: "Path to a JSON configuration file for Microsoft's profanity/chat filtering API. Primarily aimed at Realms-style hosted environments. Rarely used on self-hosted servers.",
        minVersion: "1.17"
      }
    ]
  }
]
