export const isModInstalledInInstance = (instanceMods: any[], addon: any) => {
  if (!addon || !instanceMods) return false

  const found = instanceMods.find((mod) => {
    if (addon.platform === "curseforge") {
      return mod.curseforge?.project_id === parseInt(addon.id.toString(), 10)
    } else if (addon.platform === "modrinth") {
      return mod.modrinth?.project_id === addon.id.toString()
    }
    return false
  })

  return found
}
