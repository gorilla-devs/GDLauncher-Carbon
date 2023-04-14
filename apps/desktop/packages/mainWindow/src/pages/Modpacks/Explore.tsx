import ContentWrapper from "@/components/ContentWrapper";
import { useGDNavigate } from "@/managers/NavigationManager";
import { Trans } from "@gd/i18n";

const Modpack = () => {
  const navigate = useGDNavigate();
  // const params = useParams();

  return (
    <ContentWrapper>
      {/* <div>Modpack {params.id}</div> */}
      <button onClick={() => navigate("/modpacks")}>
        <Trans
          key="instance.step_back"
          options={{
            defaultValue: "back",
          }}
        />
      </button>
    </ContentWrapper>
  );
};

export default Modpack;
