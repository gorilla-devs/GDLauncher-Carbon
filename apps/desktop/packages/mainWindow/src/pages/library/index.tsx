import InstalledInstances from "@/components/Carousels/InstalledInstances";
import PopularModpacks from "@/components/Carousels/PopularModpacks";
import RecentPlayed from "@/components/Carousels/RecentPlayed";
import Page from "@/components/Page";
import "./index.css";

const Home = () => {
  return (
    <Page class="bg-black-black p-6 noScroll overflow-auto">
      <div>
        {/* TODO: News component */}
        <div class="h-39 bg-green-400 rounded-lg" />
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
    </Page>
  );
};

export default Home;
