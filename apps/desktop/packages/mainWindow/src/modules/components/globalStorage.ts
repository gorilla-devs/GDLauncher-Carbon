import napi from "../napi";

export const init = async () => {
  await napi.initGlobalStorage();
};

export default {};