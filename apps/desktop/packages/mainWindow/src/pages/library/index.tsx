import { Carousel } from "@/components/Carousel";
import Page from "@/components/Page";
import RecentPlayed from "@/components/Carousel/RecentPlayed";
import "./index.css";
import PopularModpacks from "@/components/Carousel/PopularModpacks";

const Home = () => {
  return (
    <Page class="bg-black-black p-6 noScroll overflow-auto">
      <div>
        {/* TODO: News component */}
        <div class="h-39 bg-green-400 rounded-lg"></div>
        <div class="mt-4">
          <RecentPlayed />
        </div>
        <div class="mt-4">
          <PopularModpacks />
        </div>
        <div class="mt-4">
          <PopularModpacks />
        </div>
      </div>
    </Page>
  );
};

export default Home;
