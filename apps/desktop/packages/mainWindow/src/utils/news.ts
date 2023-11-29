import axios from "axios";
import { MOJANG_API, NEWS_URL } from "@/constants";

export interface NewsItem {
  title: string;
  category: string;
  date: string;
  text: string;
  playPageImage: PlayPageImage;
  newsPageImage: NewsPageImage;
  readMoreLink: string;
  newsType: string[];
  id: string;
  tag?: string;
  cardBorder?: boolean;
  highlight?: Highlight;
}

export interface PlayPageImage {
  title: string;
  url: string;
}

export interface NewsPageImage {
  title: string;
  url: string;
  dimensions: Dimensions;
}

export interface Dimensions {
  width: number;
  height: number;
}

export interface Highlight {
  image: Image;
  iconImage: IconImage;
  platforms: string[];
  entitlements: any[];
  title: string;
  description: string;
  until: string;
  playGame?: string;
  readMoreLink?: string;
}

export interface Image {
  url: string;
  title: string;
}

export interface IconImage {}

export const initNews = async () => {
  try {
    const { data } = await axios.get(NEWS_URL);

    const filteredNews = data.entries.filter(
      (entry: NewsItem) => entry.tag === "News"
    );

    const newsArr =
      filteredNews?.map((newsEntry: NewsItem) => ({
        title: newsEntry.title,
        description: newsEntry.text,
        image: `${MOJANG_API}${newsEntry.newsPageImage.url}`,
        url: newsEntry.readMoreLink
      })) || [];

    return newsArr.splice(0, 20);
  } catch (err) {
    console.error(err);
  }
};
