import { useNavigate } from "@solidjs/router";
import { For } from "solid-js";
import { Carousel } from ".";
import InstanceTile from "../InstanceTile";

const MockCarousel = [
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "ABDFEAD",
  },
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "DDAEDF",
  },
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "HDHEJA",
  },
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "HUSER",
  },
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "PDODK",
  },
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "AKFBI",
  },
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "AHUUIO",
  },
  {
    title: "Minecraft forge",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "HFHDJ",
  },
];

const InstalledInstances = () => {
  const navigate = useNavigate();

  return (
    <Carousel title="Your Instances">
      <For each={MockCarousel}>
        {(instance) => (
          <div id={instance.id}>
            <InstanceTile
              onClick={() => navigate(`/library/${instance.id}`)}
              title={instance.title}
              modloader={instance.modloader}
              version={instance.mcVersion}
            />
          </div>
        )}
      </For>
    </Carousel>
  );
};

export default InstalledInstances;
