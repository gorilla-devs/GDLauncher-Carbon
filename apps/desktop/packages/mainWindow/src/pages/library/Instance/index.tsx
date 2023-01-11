import { Outlet, useNavigate, useParams } from "@solidjs/router";

const Instance = () => {
  const navigate = useNavigate();
  const { id } = useParams();

  return (
    <div>
      <div class="flex h-full">
        <button onClick={() => navigate(`/library/${id}`)}>Overview</button>
        <button onClick={() => navigate(`/library/${id}/mods`)}>Mods</button>
        <button onClick={() => navigate(`/library/${id}/resourcepacks`)}>
          Resource Packs
        </button>
      </div>

      <Outlet />
    </div>
  );
};

export default Instance;
