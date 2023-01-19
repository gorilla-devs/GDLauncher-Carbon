export const getActualUrl = (url: string) => {
  const split = url.split("index.html#");
  const { pathname, search } = new URL(`http://bruh.gdlauncher.com${split[1]}`);
  return `${pathname}${search}`;
};
