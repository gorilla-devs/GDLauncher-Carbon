import ContentWrapper from "@/components/ContentWrapper";
import { useGdNavigation } from "@/managers/NavigationManager";
import { Trans } from "@gd/i18n";

const Modpack = () => {
  const navigate = useGdNavigation();
  // const params = useParams();

  return (
    <ContentWrapper>
      {/* <div>Modpack {params.id}</div> */}
      <button onClick={() => navigate?.navigate("/modpacks")}>
        <Trans
          key="back"
          options={{
            defaultValue: "back",
          }}
        />
      </button>
    </ContentWrapper>
  );
};

export default Modpack;
