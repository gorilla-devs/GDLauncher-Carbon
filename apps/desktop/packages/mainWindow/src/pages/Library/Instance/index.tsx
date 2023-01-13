import { Tabs, TabList, Tab, TabPanel } from "@gd/ui";
import { Outlet, useNavigate, useParams } from "@solidjs/router";
import { createSignal } from "solid-js";

const Instance = () => {
  const [index, setIndex] = createSignal(1);
  const navigate = useNavigate();
  const { id } = useParams();

  return (
    <div>
      <Tabs index={index()}>
        <TabList>
          <Tab onClick={() => navigate(`/library/${id}`)}>Overview</Tab>
          <Tab onClick={() => navigate(`/library/${id}/mods`)}>Mods</Tab>
          <Tab onClick={() => navigate(`/library/${id}/mods`)}>
            Resource Packs
          </Tab>
        </TabList>
        <TabPanel>
          <Outlet />
        </TabPanel>
        <TabPanel>
          <Outlet />
        </TabPanel>
        <TabPanel>
          <Outlet />
        </TabPanel>
      </Tabs>
    </div>
  );
};

export default Instance;
