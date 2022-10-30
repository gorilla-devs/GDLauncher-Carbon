import { APP_URLS } from "../constants";

export default function composeCDNAssetLink(asset: string): string {
  return `${APP_URLS.cdn}/assets/${asset}`;
}

export function composeCDNArticleThumbnailAssetLink(slug: string, img: string): string {
  return `${APP_URLS.cdn}/articles/${slug}/${img}`;
}
