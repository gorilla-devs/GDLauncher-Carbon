import { APP_URLS } from "@/constants";

export default function composeCDNAssetLink(asset: string): string {
  return `${APP_URLS.cdn}/assets/${asset}`;
}
