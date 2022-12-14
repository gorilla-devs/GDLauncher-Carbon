import InstalledInstances from "@/components/Carousels/InstalledInstances";
import PopularModpacks from "@/components/Carousels/PopularModpacks";
import RecentPlayed from "@/components/Carousels/RecentPlayed";
import { News } from "@gd/ui";
import "./index.css";

const newsArticles = [
  {
    title: "title",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
    guid: "843292n",
  },
  {
    title: "title1",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
    guid: "843292n",
  },
  {
    title: "title2",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
    guid: "843292n",
  },
  {
    title: "title3",
    description: "this is a nice and fair description",
    image: `https://www.minecraft.net/content/dam/games/minecraft/screenshots/1.19.3-rc3_1x1.jpg`,
    url: "https://randomurl.com",
    guid: "843292n",
  },
];

const Home = () => {
  return (
    <div class="p-6">
      <div>
        <News slides={newsArticles} />
        <div class="mt-4">
          <RecentPlayed />
        </div>
        <div class="mt-4">
          <PopularModpacks />
        </div>
        <div class="mt-4">
          <InstalledInstances />
        </div>
      </div>
    </div>
  );
};

export default Home;
