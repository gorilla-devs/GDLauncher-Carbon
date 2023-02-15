import { InitNews } from "@/utils/news";

const fetchData = () => {
  const news = InitNews();
  return news;
};

export default fetchData;
