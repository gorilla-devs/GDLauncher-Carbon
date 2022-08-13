import { children } from "solid-js";

enum sectionType {
  vanilla = "vanilla",
  forge = "forge",
  fabric = "fabric",
  none = "none",
}

function Section() {
  //   const c = children(() => props.children);
  return (
    <div class="h-screen">
      <div></div>
    </div>
  );
}

function ModLoaderSection() {
  //   const c = children(() => props.children);
  return (
    <div class="h-screen">
      <div class="flex flex-row">
        <h2>Vanilla</h2>
        <h2>Forge</h2>
        <h2>Fabric</h2>
        <h2>Fabric</h2>
      </div>
    </div>
  );
}

export default ModLoaderSection;
