import { Button } from "@gd/ui";

const BeginExport = () => {
  return (
    <div class="flex justify-between items-center w-full pt-4">
      <Button type="secondary" size="large">
        Cancel
      </Button>
      <Button type="primary" size="large">
        Begin Export
      </Button>
    </div>
  );
};
export default BeginExport;
