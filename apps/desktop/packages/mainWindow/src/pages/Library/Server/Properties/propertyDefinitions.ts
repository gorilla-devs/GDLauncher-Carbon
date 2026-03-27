export interface PropertyDefinition {
  key: string
  label: string
  type: "string" | "number" | "boolean" | "enum"
  defaultValue: string
  enumValues?: string[]
  min?: number
  max?: number
  description?: string
}

export interface PropertyGroup {
  id: string
  label: string
  icon: string
  properties: PropertyDefinition[]
}

export const propertyGroups: PropertyGroup[] = [
  {
    id: "gameplay",
    label: "Gameplay",
    icon: "i-hugeicons:joystick-03",
    properties: [
      { key: "difficulty", label: "Difficulty", type: "enum", defaultValue: "easy", enumValues: ["peaceful", "easy", "normal", "hard"], description: "Server difficulty level" },
      { key: "gamemode", label: "Default Gamemode", type: "enum", defaultValue: "survival", enumValues: ["survival", "creative", "adventure", "spectator"] },
      { key: "force-gamemode", label: "Force Gamemode", type: "boolean", defaultValue: "false", description: "Force players to join in default gamemode" },
      { key: "hardcore", label: "Hardcore", type: "boolean", defaultValue: "false" },
      { key: "pvp", label: "PvP", type: "boolean", defaultValue: "true" },
      { key: "spawn-protection", label: "Spawn Protection", type: "number", defaultValue: "16", min: 0, max: 1000, description: "Radius of spawn protection (0 = disabled)" },
      { key: "enable-command-block", label: "Command Blocks", type: "boolean", defaultValue: "false" },
      { key: "spawn-monsters", label: "Spawn Monsters", type: "boolean", defaultValue: "true" },
      { key: "spawn-animals", label: "Spawn Animals", type: "boolean", defaultValue: "true" },
      { key: "spawn-npcs", label: "Spawn NPCs", type: "boolean", defaultValue: "true" },
      { key: "allow-flight", label: "Allow Flight", type: "boolean", defaultValue: "false" },
      { key: "allow-nether", label: "Allow Nether", type: "boolean", defaultValue: "true" },
      { key: "player-idle-timeout", label: "Idle Timeout (min)", type: "number", defaultValue: "0", min: 0, max: 1440, description: "Kick idle players after N minutes (0 = disabled)" }
    ]
  },
  {
    id: "world",
    label: "World",
    icon: "i-hugeicons:earth",
    properties: [
      { key: "level-seed", label: "World Seed", type: "string", defaultValue: "" },
      { key: "level-type", label: "World Type", type: "enum", defaultValue: "minecraft\\:normal", enumValues: ["minecraft\\:normal", "minecraft\\:flat", "minecraft\\:large_biomes", "minecraft\\:amplified", "minecraft\\:single_biome_surface"] },
      { key: "generate-structures", label: "Generate Structures", type: "boolean", defaultValue: "true" },
      { key: "max-world-size", label: "Max World Size", type: "number", defaultValue: "29999984", min: 1, max: 29999984 },
      { key: "max-build-height", label: "Max Build Height", type: "number", defaultValue: "256", min: 64, max: 4096 },
      { key: "level-name", label: "World Name", type: "string", defaultValue: "world" }
    ]
  },
  {
    id: "network",
    label: "Network",
    icon: "i-hugeicons:wifi-01",
    properties: [
      { key: "server-port", label: "Server Port", type: "number", defaultValue: "25565", min: 1, max: 65535 },
      { key: "motd", label: "MOTD", type: "string", defaultValue: "A Minecraft Server" },
      { key: "max-players", label: "Max Players", type: "number", defaultValue: "20", min: 1, max: 1000 },
      { key: "online-mode", label: "Online Mode", type: "boolean", defaultValue: "true", description: "Require authentication with Mojang servers" },
      { key: "server-ip", label: "Server IP", type: "string", defaultValue: "", description: "Leave empty to bind all interfaces" },
      { key: "enable-rcon", label: "Enable RCON", type: "boolean", defaultValue: "false" },
      { key: "rcon.port", label: "RCON Port", type: "number", defaultValue: "25575", min: 1, max: 65535 },
      { key: "rcon.password", label: "RCON Password", type: "string", defaultValue: "" },
      { key: "enable-query", label: "Enable Query", type: "boolean", defaultValue: "false" },
      { key: "query.port", label: "Query Port", type: "number", defaultValue: "25565", min: 1, max: 65535 },
      { key: "network-compression-threshold", label: "Compression Threshold", type: "number", defaultValue: "256", min: -1, max: 65535, description: "-1 to disable compression" }
    ]
  },
  {
    id: "advanced",
    label: "Advanced",
    icon: "i-hugeicons:wrench-01",
    properties: [
      { key: "view-distance", label: "View Distance", type: "number", defaultValue: "10", min: 2, max: 32 },
      { key: "simulation-distance", label: "Simulation Distance", type: "number", defaultValue: "10", min: 2, max: 32 },
      { key: "entity-broadcast-range-percentage", label: "Entity Broadcast Range %", type: "number", defaultValue: "100", min: 10, max: 1000 },
      { key: "rate-limit", label: "Rate Limit", type: "number", defaultValue: "0", min: 0, max: 1000, description: "Max packets per second (0 = disabled)" },
      { key: "max-tick-time", label: "Max Tick Time (ms)", type: "number", defaultValue: "60000", min: -1, max: 600000, description: "-1 to disable watchdog" },
      { key: "op-permission-level", label: "OP Permission Level", type: "enum", defaultValue: "4", enumValues: ["1", "2", "3", "4"] },
      { key: "function-permission-level", label: "Function Permission Level", type: "enum", defaultValue: "2", enumValues: ["1", "2", "3", "4"] },
      { key: "white-list", label: "Whitelist", type: "boolean", defaultValue: "false" },
      { key: "enforce-whitelist", label: "Enforce Whitelist", type: "boolean", defaultValue: "false", description: "Kick non-whitelisted players when whitelist is reloaded" },
      { key: "enforce-secure-profile", label: "Enforce Secure Profile", type: "boolean", defaultValue: "true" }
    ]
  }
]
