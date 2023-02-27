import { useGdNavigation } from "@/managers/NavigationManager";

export default function About() {
  const navigate = useGdNavigation();

  return (
    <div>
      <div>
        <button onClick={() => navigate?.navigate("/modpacks/DDLTR")}>
          {/* Modpack DDLTR */}
        </button>
      </div>
    </div>
  );
}
