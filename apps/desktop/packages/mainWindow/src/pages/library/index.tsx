import InstalledInstances from "@/components/Carousels/InstalledInstances";
import PopularModpacks from "@/components/Carousels/PopularModpacks";
import RecentPlayed from "@/components/Carousels/RecentPlayed";
import "./index.css";

const Home = () => {
  return (
    <div>
      <div>
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
    </div>
  );
};

export default Home;
