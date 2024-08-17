import { NEWS_URL } from "./constants";

export interface NewsItem {
  title: string;
  description: string;
  image: string;
  url: string;
}

export const initNews = async () => {
  try {
    const resp = await fetch(NEWS_URL);
    const textData = await resp.text(); 

    const parser = new DOMParser();
    const xmlDoc = parser.parseFromString(textData, "text/xml");

    const items = xmlDoc.querySelectorAll("item");
    const newsArr: NewsItem[] = [];

    items.forEach((item) => {
      const title = item.querySelector("title")?.textContent || "";
      const description = item.querySelector("description")?.textContent || "";
      const imageUrl = item.querySelector("imageURL")?.textContent || "";
      const url = item.querySelector("link")?.textContent || "";

      // Construct the full image URL
      const fullImageUrl = `https://www.minecraft.net${imageUrl}`;

      newsArr.push({ title, description, image: fullImageUrl, url });
    });
    // News isn't published that often, showing 20 (the previous) would result in old news.
    return newsArr.slice(0, 6); 
  } catch (err) {
    console.error(err);
  }
};
