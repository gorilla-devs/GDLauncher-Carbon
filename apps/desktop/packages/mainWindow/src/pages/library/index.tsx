import Page from "@/components/Page";
import { useNavigate } from "@solidjs/router";

const Home = () => {
  const navigate = useNavigate();

  return (
    <Page class="bg-black-black">
      <button onClick={() => navigate("?m=privacyPolicy")}>Open modal</button>
      <button
        onClick={async () => {
          navigate("/library/AXDLO");
        }}
      >
        Auth
      </button>
    </Page>
  );
};

export default Home;
