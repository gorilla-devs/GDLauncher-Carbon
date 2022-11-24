import { Carousel } from "@/components/Carousel";
import Page from "@/components/Page";
import "./index.css";

const Home = () => {
  return (
    <Page class="bg-black-black p-6 max-w-200 noScroll overflow-auto">
      <div>
        {/* TODO: News component */}
        <div class="h-39 bg-green-400 rounded-lg"></div>
        <div class="mt-4">
          <Carousel title="Recent Played" />
        </div>
        <div class="mt-4">
          <Carousel title="Your Instances" />
        </div>
        <div class="mt-4">
          <Carousel title="Popular Modpacks" />
        </div>
      </div>
    </Page>
  );
};

export default Home;
