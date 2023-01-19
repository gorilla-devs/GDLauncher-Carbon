import ContentWrapper from "@/components/ContentWrapper";
import { Trans } from "@gd/i18n";
import { useNavigate } from "@solidjs/router";

const Modpack = () => {
  const navigate = useNavigate();
  // const params = useParams();

  return (
    <ContentWrapper>
      {/* <div>Modpack {params.id}</div> */}
      <button onClick={() => navigate("/modpacks")}>
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
