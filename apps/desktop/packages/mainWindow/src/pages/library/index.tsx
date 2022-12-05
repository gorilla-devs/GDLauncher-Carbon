import Tile from "@/components/Instance/Tile";
import Page from "@/components/Page";
import { Carousel } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { For } from "solid-js";
import "./index.css";

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

const Home = () => {
  const navigate = useNavigate();

  return (
    <Page class="bg-black-black p-6 noScroll overflow-auto">
      <div>
        {/* TODO: News component */}
        <div class="h-39 bg-green-400 rounded-lg" />
        <div class="mt-4">
          <Carousel title="Recent played">
            <For each={MockCarousel}>
              {(instance) => (
                <div id={instance.id}>
                  <Tile
                    onClick={() => navigate(`/library/${instance.id}`)}
                    title={instance.title}
                    modloader={instance.modloader}
                    version={instance.mcVersion}
                  />
                </div>
              )}
            </For>
          </Carousel>
        </div>
        <div class="mt-4">
          <Carousel title="Your Instances">
            <For each={MockCarousel}>
              {(instance) => (
                <div id={instance.id}>
                  <Tile
                    onClick={() => navigate(`/library/${instance.id}`)}
                    title={instance.title}
                    modloader={instance.modloader}
                    version={instance.mcVersion}
                  />
                </div>
              )}
            </For>
          </Carousel>
        </div>
        <div class="mt-4">
          <Carousel title="Popular Modpacks">
            <For each={MockCarousel}>
              {(instance) => (
                <div id={instance.id}>
                  <Tile
                    onClick={() => navigate(`/library/${instance.id}`)}
                    title={instance.title}
                    modloader={instance.modloader}
                    version={instance.mcVersion}
                  />
                </div>
              )}
            </For>
          </Carousel>
        </div>
      </div>
    </Page>
  );
};

export default Home;
