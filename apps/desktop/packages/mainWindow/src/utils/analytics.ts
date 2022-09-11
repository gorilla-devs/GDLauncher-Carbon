import pgk from "../../../../package.json";

function initAnalytics(this: any) {
  const httpsDesktopUrl = "https://" + import.meta.env.VITE_DESKTOP_APP_URL;
  var i: any = {};
  var a = window.location,
    r = window.document,
    o = r.currentScript,
    s = import.meta.env.VITE_ANALYTICS_ENDPOINT;

  function t(t: string, e: { props: any; meta?: any; callback?: any }) {
    if (
      /^localhost$|^127(\.[0-9]+){0,2}\.[0-9]+$|^\[::1?\]$/.test(a.hostname)
    ) {
      return;
    }
    (i.n = t),
      (i.d = import.meta.env.VITE_DESKTOP_APP_URL),
      (i.r = httpsDesktopUrl), // referrer
      (i.w = window.innerWidth),
      e && e.meta && (i.m = JSON.stringify(e.meta)),
      e && e.props && (i.p = e.props);
    var n = new XMLHttpRequest();
    n.open("POST", s, !0),
      n.setRequestHeader("Content-Type", "text/plain"),
      n.send(JSON.stringify(i)),
      (n.onreadystatechange = function () {
        4 === n.readyState && e && e.callback && e.callback();
      });
  }
  var e = (window.plausible && window.plausible.q) || [];
  window.plausible = t;
  for (var i, n = 0; n < e.length; n++) t.apply(this, e[n]);

  function p() {
    const sliced = a.hash.slice(1);
    const url = new URL(httpsDesktopUrl + sliced);
    const params = new URLSearchParams(url.search);
    if (params.has("m")) {
      i.u = httpsDesktopUrl + `/modal/${params.get("m")}`;
    } else {
      i.u = httpsDesktopUrl + sliced;
    }
    t("pageview", { props: { version: pgk.version } });
  }
  window.addEventListener("hashchange", p);
  p();
}

export default initAnalytics;
