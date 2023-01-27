import { Trans } from "@gd/i18n";
import { Button, Dropdown, Input } from "@gd/ui";

const Mods = () => {
  return (
    <div class="flex flex-col gap-4 max-w-185">
      <div class="flex">
        <Input
          placeholder="Type Here"
          icon={<div class="i-ri:search-line" />}
          class="w-full rounded-full text-shade-0"
        />
        <div class="flex items-center gap-4">
          <p class="m-0 text-shade-5">
            <Trans
              key="sort_by"
              options={{
                defaultValue: "Sort by:",
              }}
            />
          </p>
          <Dropdown
            rounded
            options={[
              { label: "A to Z", key: "asc" },
              { label: "Z to A", key: "desc" },
            ]}
            value="asc"
          />
          <Button variant="outline">
            <Trans
              key="add_mod"
              options={{
                defaultValue: "+ Add Mod",
              }}
            />
          </Button>
        </div>
      </div>
    </div>
  );
};

export default Mods;
