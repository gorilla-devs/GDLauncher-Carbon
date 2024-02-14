import { useContextCascader } from "./CascaderContext";

const ContextCascader = () => {
  const contextCascader = useContextCascader();
  return <div>Cascader</div>;
};
export default ContextCascader;
