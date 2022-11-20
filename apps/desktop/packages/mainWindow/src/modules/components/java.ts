import napi from "../napi";

export const init = async () => {
  let availableJavas = await napi.initJava();
  console.log(availableJavas);
  return availableJavas;
};

export default {};