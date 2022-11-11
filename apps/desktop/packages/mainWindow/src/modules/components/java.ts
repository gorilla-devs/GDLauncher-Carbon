import napi from "../napi";

export const init = async () => {
  await napi.initJava();
};

export default {};