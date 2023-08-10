import { XMLParser } from "fast-xml-parser";
import axios from "axios";
import { NEWS_URL } from "@/constants";

type NewsItem = {
  title: string;
  link: string;
  description: string;
  imageURL: string;
  primaryTag: string;
  pubDate: string;
  guid: string;
};

const getHigResImage = (imgUrl: string) => {
  return imgUrl
    .replace("277x277", "1170x500")
    .replace("carousel", "header")
    .replace("GRID", "HEADER");
};

export const initNews = async () => {
  try {
    const { data: newsXml } = await axios.get(NEWS_URL);
    const parser = new XMLParser();
    let parsedNews = parser.parse(newsXml);

    const newsArr =
      parsedNews?.rss?.channel?.item?.map((newsEntry: NewsItem) => ({
        title: newsEntry.title,
        description: newsEntry.description,
        image: getHigResImage(`https://minecraft.net${newsEntry.imageURL}`),
        url: newsEntry.link,
      })) || [];
    return newsArr;
  } catch (err) {
    console.error(err);
  }
};
