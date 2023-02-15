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

export const initNews = async () => {
  try {
    const { data: newsXml } = await axios.get(NEWS_URL);

    const parser = new XMLParser();
    let parsedNews = parser.parse(newsXml);
    const newsArr =
      parsedNews?.rss?.channel?.item?.map((newsEntry: NewsItem) => ({
        title: newsEntry.title,
        description: newsEntry.description,
        image: `https://minecraft.net${newsEntry.imageURL}`,
        url: newsEntry.link,
      })) || [];
    return newsArr.splice(0, 10);
  } catch (err) {
    console.error(err);
  }
};
