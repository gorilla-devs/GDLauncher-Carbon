import Tile from "@/components/Instance/Tile";
import { Carousel, News } from "@gd/ui";
import { useNavigate } from "@solidjs/router";
import { For } from "solid-js";
import "./index.css";
import { useTransContext } from "@gd/i18n";

const mockCarousel = [
  {
    title: "Minecraft forge 1",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "ABDFEAD",
  },
  {
    title: "Minecraft forge 2",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "DDAEDF",
  },
  {
    title: "Minecraft forge 3",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "HDHEJA",
  },
  {
    title: "Minecraft forge 4",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "HUSER",
  },
  {
    title: "Minecraft forge 5",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "PDODK",
  },
  {
    title: "Minecraft forge 6",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "AKFBI",
  },
  {
    title: "Minecraft forge 7",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "AHUUIO",
  },
  {
    title: "Minecraft forge 8",
    modloader: "forge",
    mcVersion: "1.19.2",
    id: "HFHDJ",
  },
];

const newsArticles = [
  {
    title: "title",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
  },
  {
    title: "title1",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
  },
  {
    title: "title2",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
  },
  {
    title: "title3",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
  },
];

const Home = () => {
  const navigate = useNavigate();
  const [t] = useTransContext();

  return (
    <div class="p-6">
      <div>
        <News slides={newsArticles} />
        <div class="mt-4">
          <Carousel title={t("recent_played")}>
            <For each={mockCarousel}>
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
          <Carousel title={t("your_instances")}>
            <For each={mockCarousel}>
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
          <Carousel title={t("popular_modpacks")}>
            <For each={mockCarousel}>
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
    </div>
  );
};

export default Home;
