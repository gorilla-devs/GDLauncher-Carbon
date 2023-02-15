import { createSignal } from "solid-js";
import { XMLParser } from "fast-xml-parser";
import axios from "axios";
import { NEWS_URL } from "@/constants";

export interface RSSNews {
  "?xml": string;
  rss: RSS;
}

export interface RSS {
  channel: Channel;
}

export interface Channel {
  title: string;
  link: string;
  description: string;
  "atom:link": string;
  language: string;
  pubDate: string;
  item: Item[];
}

export interface Item {
  title: string;
  link: string;
  description: string;
  imageURL: string;
  primaryTag: string;
  pubDate: string;
  guid: string;
}

export const [news, setNews] = createSignal([]);

export const InitNews = async () => {
  if (news.length === 0) {
    try {
      const { data: newsXml } = await axios.get(NEWS_URL);

      const parser = new XMLParser();
      let parsedNews = parser.parse(newsXml);
      const newsArr =
        parsedNews?.rss?.channel?.item?.map((newsEntry: Item) => ({
          title: newsEntry.title,
          description: newsEntry.description,
          image: `https://minecraft.net${newsEntry.imageURL}`,
          url: newsEntry.link,
        })) || [];

      setNews(newsArr.splice(0, 10));
    } catch (err) {
      console.error(err);
    }
  }
};
