import { initNews } from "@/utils/news";

const fetchData = () => {
  const news = initNews();
  return news;
};

export default fetchData;
