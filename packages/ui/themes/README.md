# Making a GDLauncher theme

A theme is a folder with a `manifest.json` and, if you want, some CSS, images, fonts, sounds and a video. This guide covers everything a theme can do and every rule it has to follow. The seven built-in themes in this folder (`main`, `pixelato`, `win95`, `inferno`, `aether`, `frost` and `silent-night`) use the same format, so you can copy one and change it. `silent-night` is a good one to copy; its `theme.css` shows the glass rule.

Themes are data. They can't run code, and they can't load anything from the internet.

## Quick start

1. Open Settings, then Appearance, and click **Open themes folder**. It is `<runtime>/themes/` inside the launcher's data folder.
2. Make a folder there with your theme's id as its name, for example `my-theme`, and put a `manifest.json` in it:

   ```json
   {
     "themeApi": 1,
     "id": "my-theme",
     "name": "My theme",
     "author": "You",
     "version": "1.0.0",
     "colors": { "primary": "255 120 80" }
   }
   ```

3. Your theme shows up in the installed themes on the Appearance page. Click it to turn it on.
4. Edit and save a file. While your theme is on, the launcher checks its folder every second or two and reloads it, so you see the change without restarting.
5. To share it, zip the contents of the folder (so `manifest.json` is at the top of the zip) and rename the zip to `my-theme.gdltheme`. Other people can click **Install theme** on the Appearance page, or drop the file onto the launcher window.

**If a theme makes the launcher hard to use**, reset it: View, then **Reset theme**, or press `Ctrl+Alt+Shift+T` (`Cmd+Option+Shift+T` on macOS). This works even when a theme has hidden the Settings button.

## Files and limits

| What            | Allowed                                                              |
| --------------- | -------------------------------------------------------------------- |
| `manifest.json` | Required, at the top of the theme. At most 256 KB.                   |
| CSS             | `.css` files. `theme.css` (or whatever `css` names) is at most 1 MB. |
| Images          | `png`, `jpg`, `jpeg`, `webp`, `avif`, `gif`, `svg`                   |
| Fonts           | `woff2`, `woff`, `ttf`, `otf`                                        |
| Sounds          | `ogg`, `mp3`, `wav`                                                  |
| Video           | `webm`, `mp4`                                                        |
| Text            | `txt`, at most 256 KB each. See below.                               |
| The whole theme | At most 50 MB and 2,000 files.                                       |

Any other file type makes the theme invalid.

**Licence files.** A font's licence usually has to travel with the font, so a theme can include `.txt` files, like `fonts/OFL.txt`. They're installed and kept with the theme, but never used: the manifest and the CSS can't point at them, and the launcher never serves them as CSS or as a picture.

- **Paths** are relative to the theme folder and use `/`, like `assets/hat.svg`. Letters must match exactly: `Hat.svg` is not `hat.svg`.
- **No `..`**, no absolute paths, no backslashes or colons, and no symlinks or links of any kind.
- **No Windows-reserved names** (`con`, `prn`, `aux`, `nul`, `com1` to `com9`, `lpt1` to `lpt9`), with or without an extension. A file or folder name can't end in a dot or a space. A whole path is at most 512 characters and each folder or file name in it at most 255. The id itself can't be a reserved name either.
- **Two names that differ only by upper and lower case** (`A.png` and `a.png`) are refused, because Windows and macOS would merge them.
- **The id** is 1 to 64 characters of `a-z`, `0-9` and single `-`, and doesn't start or end with `-`. For a theme in the themes folder, the folder name must be the same as the id.
- Files and folders that start with a dot, `__MACOSX`, `Thumbs.db`, `desktop.ini` and the temporary files editors leave behind are skipped.

## Manifest reference

Keys you leave out use the defaults shown. A key the launcher doesn't know is ignored and listed as a warning (see [Troubleshooting](#troubleshooting)), so a newer theme still loads on an older launcher. A value that is out of range, a missing required key, or a path to a file that isn't there makes the theme invalid.

### The top level

| Key            | Type             | Notes                                                                                                                                            |
| -------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `themeApi`     | whole number, 1  | Required. A higher number means "needs a newer launcher" and the theme is refused. The launcher currently supports 1.                            |
| `id`           | text             | Required. See the id rules above.                                                                                                                |
| `name`         | text, 1 to 64    | Required.                                                                                                                                        |
| `author`       | text, 1 to 64    | Required.                                                                                                                                        |
| `version`      | text, 1 to 32    | Required.                                                                                                                                        |
| `description`  | text, up to 300  | Optional.                                                                                                                                        |
| `names`        | object           | Translated `name`, 1 to 64 each. Keys are app languages: `english`, `italian`, `japanese`, `russian`, `spanish`, `simplified_chinese`, `french`. |
| `descriptions` | object           | Same keys as `names`; up to 300 characters each.                                                                                                 |
| `preview`      | image path       | The picture shown on the Appearance page.                                                                                                        |
| `colors`       | object           | Palette colours, see below.                                                                                                                      |
| `fonts`        | list             | Fonts to load, at most 32.                                                                                                                       |
| `tokens`       | object           | Font stacks and corner roundness.                                                                                                                |
| `css`          | `.css` file path | Your CSS, usually `theme.css`. A `theme.css` file that no `css` key points to is not used (and warns).                                           |
| `background`   | object           | One picture or video behind the whole window.                                                                                                    |
| `decorations`  | list             | Images attached to parts of the page, at most 32.                                                                                                |
| `effects`      | list             | Built-in snow, string lights and particles, at most 4 (one of each type).                                                                        |
| `layout`       | object           | Navigation, panels and tile style.                                                                                                               |
| `sounds`       | object           | Audio for app events.                                                                                                                            |
| `season`       | object           | Built-in themes only. Installed themes can't have a season: the key is ignored and warns.                                                        |

### `colors`

Each key is a palette token and each value is three numbers from 0 to 255 as text, `"r g b"`, like `"30 33 41"`. Tokens you leave out keep the default theme's colour. A name that isn't a token is ignored with a warning. There are 86 tokens:

```
accent, primary
darkSlate-50, darkSlate-100, darkSlate-200, darkSlate-300, darkSlate-400, darkSlate-500, darkSlate-600, darkSlate-700, darkSlate-800, darkSlate-900
lightSlate-50, lightSlate-100, lightSlate-200, lightSlate-300, lightSlate-400, lightSlate-500, lightSlate-600, lightSlate-700, lightSlate-800, lightSlate-900
darkGray-50, darkGray-100, darkGray-200, darkGray-300, darkGray-400, darkGray-500, darkGray-600, darkGray-700, darkGray-800, darkGray-900
lightGray-50, lightGray-100, lightGray-200, lightGray-300, lightGray-400, lightGray-500, lightGray-600, lightGray-700, lightGray-800, lightGray-900
primary-50, primary-100, primary-200, primary-300, primary-400, primary-500, primary-600, primary-700, primary-800, primary-900
red-50, red-100, red-200, red-300, red-400, red-500, red-600, red-700, red-800, red-900
yellow-50, yellow-100, yellow-200, yellow-300, yellow-400, yellow-500, yellow-600, yellow-700, yellow-800, yellow-900
green-50, green-100, green-200, green-300, green-400, green-500, green-600, green-700, green-800, green-900
brands-curseforge, brands-modrinth, brands-discord, brands-bisecthosting
```

The `darkSlate` shades are mostly the launcher's backgrounds and the `lightSlate` shades its text; start with those.

### `fonts`

Each entry loads one font file from your theme.

| Key      | Notes                                                                         |
| -------- | ----------------------------------------------------------------------------- |
| `family` | Required. 1 to 64 characters, no quotes, brackets, semicolons or backslashes. |
| `src`    | Required. A `woff2`, `woff`, `ttf` or `otf` file in the theme.                |
| `weight` | `normal`, `bold`, a number like `"400"`, or a range like `"100 900"`.         |
| `style`  | `normal`, `italic` or `oblique`.                                              |

### `tokens`

| Key            | Notes                                                                                                                             |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `font`         | The text font stack, up to 300 characters, without `;` `{` `}` `(` `)` `\` `<` `>`. Use a `family` you loaded, with fallbacks.    |
| `font-mono`    | The same, for code-like text.                                                                                                     |
| `radius-scale` | A number from 0 to 4 that multiplies every rounded corner. Default 1. `0` makes square corners and `2` makes them twice as round. |

### `background`

One layer behind the whole window. It can be an image or a muted, looping video. Where the theme has a background, the launcher makes the window's own backdrop and the frame around the panel (`panel-frame`) see-through. The panel, the pages and the bars keep their colours: for the picture to show behind them, make them see-through in your CSS.

| Key        | Notes                                                                                                           |
| ---------- | --------------------------------------------------------------------------------------------------------------- |
| `src`      | Required. An image, or a `webm` or `mp4` video.                                                                 |
| `motion`   | Up to 3 images drawn over `src` only while motion is running. Put animated SVGs here (see below).               |
| `fit`      | `cover` (default), `contain`, `fill` or `none`.                                                                 |
| `position` | `center` (default), `top`, `bottom`, `left`, `right`, `top left`, `top right`, `bottom left` or `bottom right`. |
| `dim`      | 0 to 0.9. Default 0. Darkens the picture so text reads better.                                                  |
| `blur`     | 0 to 40, in px. Default 0. Blurs `src` only, not the `motion` layers.                                           |

`fit` and `position` apply to the `motion` layers too. With `fit` `fill`, an SVG still keeps its own proportions, and so leaves bars at the sides, unless its root element has `preserveAspectRatio="none"`.

**Animated images.** An animated SVG, GIF, WebP or APNG used as `background.src` or in a decoration keeps moving even when the person has turned on reduced motion, because the browser can't pause an image. The launcher warns about it. Put the moving part in `background.motion` instead: those layers are only drawn while motion is `run`, and the still picture in `src` stays underneath. A video `src` is different: it plays only under `run`, stays paused while the window is hidden or a game is running, and goes back to its first frame under reduced motion. Animated SVGs are only used as images, so scripts inside them never run.

### `layout`

| Key         | Notes                                                                                                                                                             |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `nav`       | `standard` (default, a 60 px bar on top), `compact` (a 48 px bar with a smaller logo) or `rail` (a 64 px icon bar on the left).                                   |
| `panels`    | `true` (default) draws page content on panels. `false` draws it straight on the background.                                                                       |
| `tileStyle` | `overlay` (default) puts the name over the picture. `caption` puts the name and details under it.                                                                 |
| `tileSize`  | A whole number from 1 to 5. The library's default tile size. A size the person has picked in the library's filter menu always wins, until they reset the filters. |

The layout never changes where the ad goes, in any setting. See [Safety](#safety-and-troubleshooting).

### `decorations`

A list of images the launcher draws on parts of the page. See [Decorations](#decorations-1) for what the values mean.

#### Decoration entries

| Key      | Notes                                                                                                                  |
| -------- | ---------------------------------------------------------------------------------------------------------------------- |
| `anchor` | Required. A part of the page, from the anchor table. An anchor that isn't in the table warns and the entry is ignored. |
| `edge`   | Required. `top`, `bottom`, `left`, `right`, `top-left`, `top-right`, `bottom-left` or `bottom-right`.                  |
| `x`      | An offset: a number of px from -4000 to 4000, or a percentage of the anchor as text, like `"12.5%"` (up to 1000%).     |
| `y`      | The same, for the other direction.                                                                                     |
| `width`  | The same kind of length, above 0. Required for `left`, `right` and every corner edge.                                  |
| `height` | The same kind of length, above 0. Required for `top`, `bottom` and every corner edge.                                  |
| `images` | Required. 1 to 64 image files.                                                                                         |
| `pick`   | Which image an item shows: `first` (default), `random` or `cycle`.                                                     |
| `repeat` | `none` (default), `x` or `y`.                                                                                          |

### `effects`

A list of effects the launcher draws for you. Each is an object with a `type`; use each type at most once (a second one warns and is ignored). An effect you don't name isn't drawn. See [Effects, sounds and motion](#effects-sounds-and-motion).

#### `snow`

| Key       | Default         | Range     | Notes                                     |
| --------- | --------------- | --------- | ----------------------------------------- |
| `density` | 20              | 0 to 60   | Flakes per window area.                   |
| `speed`   | 1               | 0.2 to 3  | How fast it falls.                        |
| `minSize` | 0.6             | 0.3 to 8  | Smallest flake, in px. At most `maxSize`. |
| `maxSize` | 4.2             | 0.3 to 8  | Largest flake, in px.                     |
| `wind`    | 0               | -60 to 60 | Sideways drift. Negative goes left.       |
| `color`   | `"255 255 255"` | `"r g b"` | Flake colour.                             |
| `layers`  | 3               | 1, 2 or 3 | Depth layers, far to near.                |

#### `stringLights`

| Key       | Default                       | Range               | Notes                                       |
| --------- | ----------------------------- | ------------------- | ------------------------------------------- |
| `colors`  | cream, amber, coral and green | 1 to 12 `"r g b"`   | Bulb colours, repeated along the wire.      |
| `spacing` | 162                           | 60 to 400           | Distance between bulbs, in px.              |
| `droop`   | 17                            | 4 to 40             | How far the wire sags between hooks, in px. |
| `size`    | 1.15                          | 0.5 to 2            | Bulb size multiplier.                       |
| `twinkle` | 0.4                           | 0 to 1              | How much the bulbs flicker.                 |
| `bulb`    | `teardrop`                    | `teardrop`, `round` | The bulb shape.                             |

#### `particles`

Your own pictures, drifting over the page.

| Key       | Default | Range                    | Notes                                        |
| --------- | ------- | ------------------------ | -------------------------------------------- |
| `images`  |         | 1 to 8 image files       | Required. Each particle picks one.           |
| `preset`  |         | `fall`, `float`, `drift` | Required. The kind of motion.                |
| `density` | 6       | 0 to 30                  | Particles per window area.                   |
| `speed`   | 1       | 0.2 to 3                 | Speed multiplier.                            |
| `minSize` | 14      | 4 to 96                  | Smallest particle, in px. At most `maxSize`. |
| `maxSize` | 28      | 4 to 96                  | Largest particle, in px.                     |
| `rotate`  | `true`  | `true` or `false`        | Whether particles spin as they move.         |
| `opacity` | 0.9     | 0.1 to 1                 | How solid they are.                          |

### `sounds`

Each key is an app event and each value is an audio file (`ogg`, `mp3` or `wav`) in the theme. Events you leave out stay silent.

| Key                | Plays when                                                                                                                                  |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `launch`           | A game starts launching.                                                                                                                    |
| `downloadComplete` | An instance or server install finishes (the game files, a mod, a server or a server modpack). Other downloads, such as Java, don't play it. |
| `click`            | The person clicks a primary button (one that isn't disabled).                                                                               |
| `notification`     | A notification toast appears.                                                                                                               |
| `error`            | An error toast appears.                                                                                                                     |

More about sounds, volume and when they are silent is under [Effects, sounds and motion](#effects-sounds-and-motion).

### `season`

Built-in themes can limit themselves to a time of year, with a dialog offering them. Installed themes can't: the `season` key is ignored with a warning, and the theme is always available. If you want a seasonal look in your own theme, the person turns it on and off themselves on the Appearance page.

## CSS

Your `css` file goes after the launcher's own styles, so it wins when selectors are equally specific. The launcher checks it when it loads.

### What to select

CSS reaches every element of the page. But nothing in the page is promised to stay the same between launcher releases: when we rework a page, its parts change, and so can their hooks. Some selectors survive that better than others.

- **Hooks are the best targets.** `data-gdl` hooks name the parts of the page, so they survive our reworks more often than anything else. Select one with `[data-gdl~="name"]`. The `~=` matters, because an element can have several hooks. You can put any combinator between two hooks (`[data-gdl~="panel"] [data-gdl~="button"]`, `>`, `+`, `~`), and add pseudo-classes and pseudo-elements.
- **The documented variables and `html` attributes** below, and `html` and `body`.
- **Everything else works too, but breaks more often.** Selectors that walk down from a hook (`[data-gdl~="library.toolbar"] > div`), element types (`h1`, `svg`) and our utility class names (`bg-darkSlate-800`) change whenever we touch that part of the page, without notice. Use them when no hook reaches what you want, and check your theme after each update. If you keep needing one, ask for a hook.

Silent Night, the newest built-in theme, uses only hooks, the variables and the `html` attributes, and that's our rule for the themes we ship from now on. Some older built-ins also use element selectors and `*` (Frost and Windows 95). Your theme can do more.

`*` and pseudo-elements after a hook are fine (`[data-gdl~="page-title"] *`, `[data-gdl~="panel"] ::-webkit-scrollbar-thumb`), and so are your own custom properties (`--my-ink: 40 30 20`). So are `:has()`, `:is()`, `:where()` and `:not()` with hooks inside (`[data-gdl~="settings.row"]:has([data-gdl~="switch"])`), and `@property`, which can stop one of your properties from inheriting or let a gradient's angle animate.

To tell one dialog, Settings tab or Search filter from another, use its identity token, like `[data-gdl~="modal.instance-creation"]`, instead of counting siblings. See [Identity tokens](#identity-tokens).

A good rule:

<!-- prettier-ignore -->
```css
/* Round tiles, with a coloured shadow. Hooks and variables only. */
[data-gdl~="library.tile.image"] {
  border-radius: calc(14px * var(--radius-scale));
  box-shadow: 0 8px 24px rgb(var(--primary-700) / 0.45);
}

/* The primary button, but not the red or green ones, or a disabled one. */
[data-gdl~="button.primary"]:not([data-gdl~="button.tinted"]):not([data-gdl~="button.disabled"]) {
  background: rgb(var(--primary-500));
}
```

### When the app's own style wins

The app styles most parts with one class each, so a rule with one hook beats it: your file comes later. That includes the colours the launcher sets for a layout, like the caption tiles' names, the panel frame it clears under a background picture, a tile's rounding, the lines between Settings rows (a plain `[data-gdl~="settings.row"] { border-top: … }` replaces them, and `border: none` clears them), the News slideshow's words, and the two dims behind a dialog. Where the app's rule weighs more:

- **A state the app draws with a pseudo-class or an attribute**, like a tab's hover colour or the selected tab's colour. Put `:root ` in front of your selector: `:root [data-gdl~="tab.active"]`.
- **A part that changes when you hover its parent**: a tile's picture, which zooms and blurs (`library.tile.picture`), and its dark layer (`library.tile.dim`); a search result's `search.result.backdrop`, `.panel`, `.description`, `.stats` and `.action`; a favourite's play layer (`library.dock.item.play`); and the highlights behind the logo and the account button (`nav.home.highlight`, `account-menu.highlight`). The app's rule names the parent's hover, so yours has to as well: `[data-gdl~="library.tile"]:hover [data-gdl~="library.tile.picture"] { transform: none; filter: none; }` turns off the zoom and blur, and `[data-gdl~="search.result"]:hover [data-gdl~="search.result.action"]` restyles the install button.
- **A few states use `!important`**: a tile's dark layer stays clear while it loads (`library.tile.loading`), and the tile's play button (`library.tile.play`) and a favourite's play layer (`library.dock.item.play`) stay shown while their right-click menu is open. Only `!important` changes those.
- **With panels off**, the cleared panels: see [Panels off](#panels-off).
- **Toasts**, only for what is set inline or marked important:
  - a toast's background, words and border colours, and its action and cancel buttons, are set inline. You don't need `!important` for the colours: they read palette variables, so set `--darkSlate-800` (the background), `--darkSlate-600` (the border) and `--lightSlate-50` (the words) on `toast`. The action button's words read `--gdl-on-primary`;
  - the close button's background, colour, border, shadow, corners and padding are `!important` in the app, so only `!important` changes those. Its place comes from the toast library's own rule, which two hooks beat: `[data-gdl~="toast"] [data-gdl~="toast.close"] { left: auto; right: 8px; }`.

  A toast's corners, shadow, outline, padding and `clip-path` change with a plain rule. See [Toast tips](#toast-tips) for more.

Don't use `!important` anywhere else.

### Fonts

The launcher sets the font on every element, not only on the page, so a `font-family` on a hook reaches that element's own text and not its children's. To change the whole app's font, set `tokens.font`. To change one part, set it on the hook that holds the words (`page-title`, `section-title.text`, `library.tile.name`), or on the hook and everything in it:

```css
[data-gdl~="settings.nav"],
[data-gdl~="settings.nav"] * {
  font-family: "My Font", sans-serif;
}
```

The same goes for menu and list items, whose words sit in child elements: a font on `select.item`, `dropdown.item`, `context-menu.item` or `settings.nav.item` needs `[hook] *` too.

The log and console lines set the mono font (`--font-mono`) on every element in them the same way, so style their whole box: `[data-gdl~="logs.content"] *` or `[data-gdl~="server.console"] *`.

Other text properties, like `color`, `font-size` and `letter-spacing`, are passed down as usual.

**Italics.** The launcher turns off made-up italics (`font-synthesis: none`), so a font with no italic face stays upright under `font-style: italic`. Bundle the italic face, or set `font-synthesis: style` where you want the browser to slant it.

### Variables

| Variable                | What it holds                                                                                                                                                                                                |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `--<palette token>`     | The colour as `"r g b"`, for example `--darkSlate-800`. Use it as `rgb(var(--darkSlate-800) / 0.5)`.                                                                                                         |
| `--font`, `--font-mono` | The font stacks.                                                                                                                                                                                             |
| `--radius-scale`        | The corner multiplier from `tokens`. Multiply your own radii by it so `radius-scale` still works.                                                                                                            |
| `--gdl-panel-inset`     | Set on `panel-frame`. Where the panel sits inside the frame, as a CSS `inset` value.                                                                                                                         |
| `--gdl-panel-radius`    | Set on `panel-frame`. The panel's `border-radius`.                                                                                                                                                           |
| `--gdl-tile-width`      | Set on `library.grid`, inline, so read it and don't set it. The width of a tile for the current tile size, like `184px`. The grid's columns use it.                                                          |
| `--gdl-text-lightness`  | Set on `html`, so read it and don't set it. How light your `lightSlate-50` is, from 0 (black) to 1 (white): the average of its three numbers over 255. The News slideshow switches to light words below 0.7. |
| `--ad-column-width`     | Set on `html`. The width of the ad column in px, or `0px` when there is none. Read it to keep clear of the column. Setting it changes nothing: the ad doesn't move.                                          |

These you set. Each has a default, used when you leave it out, so set only the ones you need, on `:root` or on the part they're for.

| Variable                    | What it does                                                                                                                                                                                                      |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--gdl-on-primary`          | `"r g b"`. The colour of words and icons on an accent fill: primary buttons, the filled badge (`badge.default`), a ticked box's mark, a tile's Play button and a toast's action button. Default: `lightSlate-50`. |
| `--gdl-on-primary-disabled` | `"r g b"`. The words of a disabled primary button. Default: `64 71 89`, a dark grey whatever the palette.                                                                                                         |
| `--gdl-tile-radius`         | A length. The rounding of a tile's frame and picture and of its selection outline, all at once. Default: `calc(1rem * var(--radius-scale))`. `0` makes square tiles without touching the rest of the app.         |
| `--gdl-group-bleed`         | A length. How far tiles may spill out of an open library group (shadows, hover lifts, stickers) before the group cuts them off. Default: `4px`, room for focus outlines. Set it on `library.group`.               |
| `--gdl-result-index`        | Set on each `search.result`, inline, so read it. Its place in the results, from 1, for numbering them: `counter-reset: n var(--gdl-result-index)` on the result and `content: counter(n)` on a `::before`.        |

### What the colours are used for

Each palette token has more than one job. If you change one, look at the places below. This matters most for a light theme.

| Tokens                               | Used for                                                                                                                                                        |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `darkSlate-900`                      | The darkest backgrounds: the ad column's backdrop, the log and console boxes, the dims behind a dialog (over the page and over the ad column), tooltips.        |
| `darkSlate-800`                      | The panel and the pages, the top bar, inputs, menus, the window itself and the logos on the ad column's backdrop.                                               |
| `darkSlate-700`                      | The frame around the panel, cards, tab bars and dialogs.                                                                                                        |
| `darkSlate-600`, `darkSlate-500`     | Raised and hovered backgrounds, borders, input fields and loading placeholders.                                                                                 |
| `darkSlate-300` to `darkSlate-500`   | Also text: placeholders, dim icons, the icon in a search box.                                                                                                   |
| `lightSlate-50`                      | The main text and icons, and by default the words on accent fills (`--gdl-on-primary`).                                                                         |
| `lightSlate-100` to `lightSlate-400` | Secondary text.                                                                                                                                                 |
| `lightSlate-500` to `lightSlate-800` | Dim text: descriptions, labels, hints.                                                                                                                          |
| `lightGray-800`, `lightGray-900`     | The scrollbar thumb.                                                                                                                                            |
| `primary-500`                        | The accent: primary buttons, the selected tab, ticked boxes, switches that are on, links.                                                                       |
| `primary-300`, `primary-400`         | Text in the accent colour, like counts and some links.                                                                                                          |
| `red`, `yellow`, `green`             | Errors, warnings and success. `500` is the strong colour and `300` and `400` are text. `900` is a dark background with light text on it, like the snapshot bar. |
| `brands-*`                           | The CurseForge, Modrinth, Discord and BisectHosting colours.                                                                                                    |

`darkGray`, `accent` and `primary` (without a number) are hardly used.

**Words on accent fills.** Primary buttons, the filled badge, a ticked box's mark, a tile's Play button and a toast's action button put words on `primary-500`. They use `--gdl-on-primary`, which is `lightSlate-50` unless you set it. A light theme with a dark accent sets it to a light colour: `:root { --gdl-on-primary: 255 255 255; }`. A disabled primary button has no fill of its own, only stripes, and its words use `--gdl-on-primary-disabled`.

**Parts that adapt to a light palette.** A few parts would vanish with a light palette's dark text, or were drawn in plain white, so they change with it:

- The News slideshow's words (`news.slide.title`, `.text` and `.date`) sit on a fixed dark veil (`news.slide.veil`) over the story's picture. With a dark palette they use your `lightSlate-50` (the title) and `lightSlate-200` (the summary and date). When your `lightSlate-50` is darker than 70% (`--gdl-text-lightness` below 0.7), all three switch together to the default theme's light ones.
- The CurseForge logo (`platform-logo.curseforge`) and the search box's open filter button (`nav.search.filters.open`) are white on a dark palette, as they always were, and take your `lightSlate-50` on a light one, where white would vanish.

Restyle them through their hooks if your theme wants something else.

**The dims behind a dialog.** A dialog dims the page with `modal.backdrop` and the ad column's backdrop with `modal.column-backdrop`, both in `darkSlate-900` at 95%. Restyle them together, so the column matches the page:

```css
[data-gdl~="modal.backdrop"],
[data-gdl~="modal.column-backdrop"] {
  background-color: rgb(40 30 20);
}
```

**A second palette inside one part.** Palette tokens are CSS variables, so you can set them on a hook, and the app's colours inside that part follow: `[data-gdl~="modal"] { --lightSlate-50: 34 40 58; --darkSlate-700: 250 246 236; }` gives dialogs ink on paper inside a dark theme. Text that has no colour of its own keeps the colour it already got from outside the part, so set `color` on the part itself too: `[data-gdl~="modal"] { color: rgb(var(--lightSlate-50)); }`. The same works for one icon: toast icons draw with `--green-500`, `--red-500`, `--yellow-500` or `--primary-500`, so setting that variable on `toast.icon` recolours it.

### `html` attributes

| Attribute             | Values                                                                    |
| --------------------- | ------------------------------------------------------------------------- |
| `data-gdl-theme`      | The theme's key: `silent-night`, or `user:my-theme` for an installed one. |
| `data-gdl-motion`     | `run`, `pause` or `off`. See [Motion](#motion).                           |
| `data-gdl-background` | Present when the theme has a `background`.                                |
| `data-gdl-nav`        | `standard`, `compact` or `rail`.                                          |
| `data-gdl-panels`     | `on` or `off`.                                                            |
| `data-gdl-tiles`      | `overlay` or `caption`.                                                   |

### Files in your CSS

`url()` can point at files inside your theme, and nothing else:

- Write it relative to the theme folder, not to the CSS file: `url(assets/frost.svg)`, even when the CSS is in a sub-folder. The file has to exist, with matching letter case, and be an image, font, sound or video.
- `@import` is refused.
- No remote, `data:`, `file:` or protocol-relative (`//host`) URLs, in `url()` or in any quoted string. That includes `src()` and strings inside `image-set()`.
- Inside `image-set()`, `-webkit-image-set()` and `image()` don't use `var()`, `env()`, `attr()`, `if()`, `inherit()` or any `--custom()` function. Put the `url(...)` in a custom property and use that instead.
- No query strings (`?x=1`) on a file path. `url(#id)`, which points at something in the page itself (an SVG filter or mask), is allowed.
- Anything else that can't be read as CSS makes the theme invalid, and the error names the line.

### The glass rule

Frosted glass (`backdrop-filter`) is the look many themes want. But an element with `backdrop-filter`, `filter`, `transform`, `rotate`, `translate`, `scale` or `perspective` becomes the reference box of every `position: fixed` thing inside it. The launcher has fixed things inside its panels: drag-select rectangles, drag ghosts and floating action bars. They would be offset and clipped. A `clip-path` doesn't move them, but it cuts them off at its edge.

So: **never put `backdrop-filter`, `filter`, `transform`, `rotate`, `translate`, `scale`, `perspective` or `clip-path` on an element that has fixed-position descendants.** Anywhere in a page, that means `panel-frame`, `panel`, `page`, `page.body`, `content` and the page roots like `library`. The same goes for `will-change` naming any of them, and `contain: paint` or `layout`.

Draw the glass on a layer under the panel instead. `panel-frame` gets two variables to size it: `--gdl-panel-inset` and `--gdl-panel-radius`.

```css
[data-gdl~="panel-frame"]::before {
  content: "";
  position: absolute;
  inset: var(--gdl-panel-inset);
  pointer-events: none;
  box-sizing: border-box;
  border-radius: var(--gdl-panel-radius);
  background: linear-gradient(rgb(22 37 72 / 0.78), rgb(12 22 48 / 0.78));
  backdrop-filter: blur(16px);
}

/* The panel itself stays see-through, with no filter. */
[data-gdl~="panel"] {
  background: transparent;
}
```

The nav bar and the sticky bars (`library.toolbar`, `settings.nav`, `instance.tabs`, `server.tabs`, `news.tabs`) can take a `backdrop-filter` themselves, because nothing fixed is inside them. Tooltips, menus and dropdowns are drawn outside the page, so they're fine too. Silent Night's `theme.css` is the example.

- **Blur over blur smears.** A sticky bar with its own `backdrop-filter`, over glass that is already blurred, blurs the blur: bright points of your background turn into soft blobs. Use a lighter blur on the bar, or none.
- **Bars on the panel's top edge.** The Settings tab bar (`settings.nav`) and an instance's or server's cover (`instance.header`, `server.header`) sit on the panel's top edge with their own background, so they hide what you draw under the panel there. A cut corner on the glass has to be cut on them too (the same `clip-path`), or it vanishes on those pages.
- **Textures that line up.** Give the panel and the sticky bars the same picture with `background-attachment: fixed` (`background: url(assets/paper.png) center / cover fixed`) and it lines up across them with no seam. The `background` hook can also carry CSS of its own under the picture, like a grid drawn with gradients.

### Panels off

When `layout.panels` is `false`, the launcher clears the background, border and shadow of `panel-frame`, `panel`, `page` and `page.body`, so the page sits straight on your background. Without a `background`, the page shows your `darkSlate-800`, the panel's own colour. It can't clear a pseudo-element you drew (like the glass above). Scope any panel art to `html[data-gdl-panels="on"]`:

```css
html[data-gdl-panels="on"] [data-gdl~="panel-frame"]::before {
  /* the glass */
}
```

To give a cleared part a colour back, use a selector as strong as the launcher's: `html[data-gdl-panels="off"] [data-gdl~="panel"]`.

Sticky bars (the library toolbar, the instance and server tabs, the Settings tab bar and the News tab bar) don't lose their background when panels are off. Keep them opaque enough that text stays readable over scrolled content. The News tab bar is also a `page.body`, so a rule on `page.body` reaches it too.

### Tiles

The tile sizes, in px: 1 is 96, 2 is 184, 3 is 240, 4 is 336 and 5 is 480. The picture is square, and `--gdl-tile-width` on `library.grid` holds the width of the current size, so decorations and your own rules can follow it.

`library.grid` is a CSS grid. Change its `grid-template-columns`, `gap` or `justify-content` to lay the tiles out differently. Each child of the grid holds one tile, so pick tiles by position on the children: `[data-gdl~="library.grid"] > :nth-child(3n)`. Two things to know:

- Dragging a tile to reorder assumes the 16 px gap between columns. With a wider gap, the strip between two tiles that isn't a drop target gets wider too, so a drop there does nothing.
- While you drag, the grid also holds drop zones and a preview of the dropped tile, and `:nth-child` counts them. An open folder's tiles are in a grid of their own, in a panel over the page.

A tile's name stays on one line and is cut with an ellipsis, at every size. To let it wrap:

```css
[data-gdl~="library.tile.name"] {
  white-space: normal;
}
```

A tile carries its state as extra hooks: `library.tile.running`, `library.tile.favorite`, `library.tile.selected` and the others in the table.

`--gdl-tile-width` can drive a whole layout, not only decorations: sizes, paddings and row heights worked out from it follow every tile size the person picks.

**A tile's layers**, from the outside in:

- `library.tile`: the whole tile, with the caption under the picture in `caption` style.
- `library.tile.frame`: rounds and clips everything inside it, with 2 px of padding round the picture. On an instance or server tile, the selection outline (`library.tile.outline`) and the coloured ring (`library.tile.glow`) live in it. A folder's frame is as wide as its grid cell, so its outline sits outside the frame, at the picture's size plus 2 px round it.
- `library.tile.image`: the picture's box. It clips too, and in `overlay` style the name, details, Play button, checkbox and badges all live inside it, so anything you move outside its edges is cut off. It always clips (it is a paint container, set inline), so a list row, a mat or a card round the picture means resizing or restyling this box itself, not drawing outside it.
- `library.tile.picture`: the picture itself. It zooms and blurs when you hover the tile; turn that off with `[data-gdl~="library.tile"]:hover [data-gdl~="library.tile.picture"] { transform: none; filter: none; }`.
- Over the picture: `library.tile.dim`, `library.tile.shade`, and while it installs, waits in the queue or is deleted, `library.tile.veil` (the blur), `library.tile.fade` (two dark gradients), `library.tile.progress` with its `.percent` and `.task` lines, `library.tile.spinner`, and the bar at the bottom: `library.tile.bar`, `.track`, `.fill` and `.text`.

**Rounding.** `--gdl-tile-radius` rounds the frame, the picture's box and the picture, and the selection outline follows it. Set it once (`:root { --gdl-tile-radius: 0; }`) for square tiles without changing the rest of the app's corners. On an instance or server tile the outline fills the frame, whatever size you give it; on a folder it keeps the picture's size for the tile size. It takes `stroke` and `stroke-width` from `library.tile.outline`; hide it with `display: none` to draw your own ring on `library.tile.selected`.

**Groups clip.** In the grouped library, each group cuts off whatever spills out of it, so tile shadows, hover lifts and stickers stop at its edge. Set `--gdl-group-bleed` on `library.group` (`[data-gdl~="library.group"] { --gdl-group-bleed: 24px; }`) to let them spill that far while the group is open. Or give `library.grid` padding.

### Caption tiles

With `tileStyle` `caption`, the name and details sit in a `library.tile.caption` block under the picture, so the `library.tile` box gets taller. Decorations on `library.tile` sized in `%` follow that taller box; use px if you want a fixed size. Loading placeholders and the drag preview stay square, since they have no caption.

The launcher colours caption names and details for the panel. A plain `[data-gdl~="library.tile.name"]` rule replaces those colours.

### Rail navigation

With `nav` `rail`, the rail replaces the top bar. It has no room for the search box, so there is a search button in the rail, and on the Search and addon pages a search strip above the page. On those pages **two elements carry `nav.search`**: the rail's button and the strip. A rule on `nav.search` applies to both.

### Header fades

The instance and server pages fade their cover picture into the page. The fade hooks (`instance.header.fade`, `server.header.fade`) sit on two stacked elements that both carry the hook, so a translucent `background` there adds up: two layers of 0.78 look about 0.95 solid. Use a lower alpha, or set it on one. A decoration on the bottom edge of `instance.header` or `server.header` is drawn under that fade, so the fade can hide it. The add-on page's `addon-page.header.fade` works the same way.

On the Search page, the box at the top of the filter sidebar that shows the instance or server you're searching for is `search.context`. It has a `search.context.fade` layer that fades the picture from the left, and a separate `search.context.shade` layer that fades it from the bottom. They're two layers, not one; style both.

### Section titles

`section-title` is on every Settings row title and every library group header, and it has the same shape in both: the words are in a `section-title.text` element inside it. So a rule on `section-title.text` styles the words, and one on `section-title` styles the whole title (in the library, that includes the arrow). A decoration or a `::after` on `section-title` repeats in each one, which suits a divider line better than a big picture.

To style only one of the two, add where it is: `[data-gdl~="settings.row.title"]`, or `[data-gdl~="library.group"] [data-gdl~="section-title"]`.

The rows of a Settings page sit in a `settings.rows` list, which draws the lines between them on each row's top border. A plain rule on `settings.row` replaces those lines. On Accounts, the Minecraft accounts table also sits in the list, with the same line on its own top border; it isn't a `settings.row`, so reach it with `[data-gdl~="settings.rows"] > table`.

### Toast tips

- What's inline and what isn't: see [When the app's own style wins](#when-the-apps-own-style-wins).
- The toast already uses its `::after`: an invisible strip that keeps the hover between stacked toasts. A `::after` of yours inherits its position and size, so set `inset`, `width` and `height` yourself, or use `::before`.
- The icon box (`toast.icon`) is left-aligned by the toast library; centre what's in it with `justify-content: center`.

### Control tips

- **Switches.** The app moves a switch's knob (`switch.thumb`) with its own `transform`. A wider or taller switch has to set the knob's `transform` for each state itself: `[data-gdl~="switch.checked"] [data-gdl~="switch.thumb"] { transform: translateX(28px); }`.
- **Icons are masks.** Most icons, like the favourites star, are drawn as a CSS mask filled with the text colour, so `color` recolours them. To swap one for a picture, set `mask: none` and a `background`.
- **Search's Platform choices** are radios (`radio`, `radio.checked`), not `search.filter.option`.
- **Sliders** (the Java memory slider): `slider`, `slider.track`, `slider.fill`, `slider.thumb`, `slider.mark` and `slider.label`. The knob at rest is the text colour, faint on a dark palette and stronger on a light one; on hover and while dragging the app's own colours win, so restyle those with `[data-gdl~="slider"]:hover [data-gdl~="slider.thumb"]`. Labels that would run into the one before them are hidden (`visibility: hidden`).
- **The BETA label on Appearance** is placed 40 px above the tab's name, where the icon is. A theme that hides tab icons can move it back: `[data-gdl~="settings.nav.item.appearance"] [data-gdl~="feature-badge"]`.

### Log lines

A log line (`logs.line`) holds its number (`logs.line.number`), then the parts the person has switched on: `logs.line.time`, `logs.line.source`, `logs.line.level`, `logs.line.logger` and `logs.line.thread`, and the message (`logs.line.message`). Each part paints its own `darkSlate-900` background, so a texture on `logs.content` shows only between them; clear theirs with `background: none` to let it through.

The person's Logs Density setting is on every line: `logs.line.comfortable` or `logs.line.compact`, and neither for Low. If you change the lines' padding, keep the three apart.

## Hooks

Select a hook with `[data-gdl~="name"]`. Hooks are the recommended targets: they name the parts of the page, and survive our reworks more often than anything else, though nothing is promised between releases. Some show up only in some states or on some pages, and a rule that matches nothing is harmless.

A hook that ends in a state, like `tab.active`, `library.tile.running` or `switch.checked`, sits next to its base hook (`tab`, `library.tile`, `switch`) only while that state holds, and comes and goes as it changes.

### Around every page

The window's frame: the bar or rail, the column the pages live in and the panel they're drawn on.

| Hook                             | Where it is                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `app`                            | The root wrapper of the window.                                                                                                                                                                                                                                                                                                                     |
| `nav`                            | The top bar, or the rail. Also the search strip above the page with the rail.                                                                                                                                                                                                                                                                       |
| `nav.rail`                       | The left rail, with `nav: "rail"` only.                                                                                                                                                                                                                                                                                                             |
| `nav.search`                     | The search part of the nav. With the rail: the rail's button and the strip, two elements.                                                                                                                                                                                                                                                           |
| `nav.search.box`                 | The search input box itself.                                                                                                                                                                                                                                                                                                                        |
| `nav.search.help`                | The help button (?) in the search box, which opens the search tips.                                                                                                                                                                                                                                                                                 |
| `nav.search.filters`             | The filter button in the search box, which shows or hides the Search filter sidebar.                                                                                                                                                                                                                                                                |
| `nav.search.filters.open`        | Added to that button while the sidebar is shown.                                                                                                                                                                                                                                                                                                    |
| `nav.actions`                    | The group of buttons at the right of the top bar.                                                                                                                                                                                                                                                                                                   |
| `nav.home`                       | The logo button at the left of the top bar: the back arrow and the logo. It goes to the library.                                                                                                                                                                                                                                                    |
| `nav.home.highlight`             | The layer behind it that shows on hover. It changes when you hover `nav.home`, so name that hover: `[data-gdl~="nav.home"]:hover [data-gdl~="nav.home.highlight"]`.                                                                                                                                                                                 |
| `nav.back`                       | The arrow left of the logo in the top bar. It is see-through on the library.                                                                                                                                                                                                                                                                        |
| `nav.update`                     | The icon that says an update is ready. Only while one is.                                                                                                                                                                                                                                                                                           |
| `nav.settings`                   | The Settings button in the top bar or the rail. It is also a `tab`.                                                                                                                                                                                                                                                                                 |
| `nav.news`                       | The News button in the top bar or the rail. It is also a `tab`.                                                                                                                                                                                                                                                                                     |
| `account-menu`                   | The account button and what it opens: the strip in the top bar, or the menu in the nav rail.                                                                                                                                                                                                                                                        |
| `account-menu.button`            | The focusable button around `account-menu.trigger`. It carries the keyboard focus ring, an outline in the accent colour that `[data-gdl~="account-menu.button"]:focus-visible` restyles. The ring in the open strip is `[data-gdl~="account-menu.strip"] :focus-visible`, and in the rail's menu `[data-gdl~="account-menu.popup"] :focus-visible`. |
| `account-menu.trigger`           | The account button: the head, the name and the arrow.                                                                                                                                                                                                                                                                                               |
| `account-menu.trigger.open`      | Added to the account button while its menu is open (the nav rail; in the top bar the strip takes the button's place).                                                                                                                                                                                                                               |
| `account-menu.highlight`         | The layer behind the account button that shows on hover. Like `nav.home.highlight`, name the trigger's hover to restyle it.                                                                                                                                                                                                                         |
| `account-menu.popup`             | The menu the account button opens in the nav rail. It is also a `dropdown`.                                                                                                                                                                                                                                                                         |
| `account-menu.section`           | One of the three cards in the rail's menu: the GDLauncher account, the Minecraft accounts and the manage button.                                                                                                                                                                                                                                    |
| `account-menu.section.title`     | The heading of a card, with its icon.                                                                                                                                                                                                                                                                                                               |
| `account-menu.item`              | One Minecraft account in the rail's menu.                                                                                                                                                                                                                                                                                                           |
| `account-menu.item.active`       | Added to the account in use.                                                                                                                                                                                                                                                                                                                        |
| `account-menu.manage`            | The button that opens the accounts settings: the gear in the strip, or the button in the rail's menu.                                                                                                                                                                                                                                               |
| `account-menu.strip`             | The strip the account button opens inside the top bar, in its place. Nothing in it drops below the bar.                                                                                                                                                                                                                                             |
| `account-menu.account`           | One Minecraft account in the strip: the one in use, by name, or another, as its face.                                                                                                                                                                                                                                                               |
| `account-menu.account.active`    | Added to the account in use, the first in the strip.                                                                                                                                                                                                                                                                                                |
| `account-menu.account.attention` | Added to an account that is expired, refreshing or invalid.                                                                                                                                                                                                                                                                                         |
| `account-menu.account.name`      | An account's name. On a face it is hidden until the face is hovered or focused, then shows over the bar beside it, without moving anything.                                                                                                                                                                                                         |
| `account-menu.more`              | "+N": how many more accounts there are than faces. It opens the accounts settings.                                                                                                                                                                                                                                                                  |
| `account-menu.add`               | The button that adds a Minecraft account.                                                                                                                                                                                                                                                                                                           |
| `account-menu.gdl`               | The GDLauncher account in the strip: its name, or that none is synced. It opens the accounts settings.                                                                                                                                                                                                                                              |
| `account-menu.close`             | The arrow that closes the strip.                                                                                                                                                                                                                                                                                                                    |
| `nav.logo`                       | The launcher logo in the nav.                                                                                                                                                                                                                                                                                                                       |
| `content`                        | The column under the nav and left of the ads. All pages, effects and decorations live in it.                                                                                                                                                                                                                                                        |
| `ad-column-background`           | The backdrop of the ad column. Its background is the column's colour, so a `background` here replaces it. Only its background may be styled; nothing may cover, hide, fade, move or imitate the ad. The ad is a separate protected layer.                                                                                                           |
| `ad-column-background.pattern`   | The logo pattern on the column backdrop, drawn when the theme has no background picture. Its `color` is the logos' colour (the marks inside them stay `darkSlate-900`); hide it with `display: none`.                                                                                                                                               |
| `release-bar`                    | The red or yellow bar above the library that says this is a snapshot, beta or alpha build. Not on the normal releases.                                                                                                                                                                                                                              |
| `release-bar.snapshot`           | Added on a snapshot build.                                                                                                                                                                                                                                                                                                                          |
| `release-bar.beta`               | Added on a beta build.                                                                                                                                                                                                                                                                                                                              |
| `release-bar.alpha`              | Added on an alpha build.                                                                                                                                                                                                                                                                                                                            |
| `release-bar.icon`               | The warning icon in that bar.                                                                                                                                                                                                                                                                                                                       |
| `feature-badge`                  | A small BETA, NEW or SOON label, like the ones on the Servers tab, the Logs tab and Appearance.                                                                                                                                                                                                                                                     |
| `feature-badge.new`              | Added to a NEW label (not used yet).                                                                                                                                                                                                                                                                                                                |
| `feature-badge.beta`             | Added to a BETA label.                                                                                                                                                                                                                                                                                                                              |
| `feature-badge.soon`             | Added to a SOON label (not used yet).                                                                                                                                                                                                                                                                                                               |
| `panel-frame`                    | The frame around a page's panel. Carries the two panel variables. With a background picture, the launcher makes it see-through.                                                                                                                                                                                                                     |
| `panel`                          | The rounded panel pages are drawn on.                                                                                                                                                                                                                                                                                                               |
| `page`                           | The scrolling container of a page (the instance and server pages).                                                                                                                                                                                                                                                                                  |
| `page.body`                      | The body area of a page, and some side panels.                                                                                                                                                                                                                                                                                                      |
| `page-title`                     | The element that holds a page's title text: Settings pages (`h3`), and the name on the instance, server and add-on pages (`h1`). The instance's name is edited in place, so with see-through gradient text set `caret-color` too.                                                                                                                   |
| `section-title`                  | A section title: every Settings title and every library group header. It holds a `section-title.text` element.                                                                                                                                                                                                                                      |
| `section-title.text`             | The words of a section title, inside `section-title`.                                                                                                                                                                                                                                                                                               |

### Library

The library page, its tiles and the favourites dock.

| Hook                            | Where it is                                                                                                                                                                                                                                                                                                                              |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `library`                       | The whole library page.                                                                                                                                                                                                                                                                                                                  |
| `library.toolbar`               | The bar above the library with its tabs, search and filters.                                                                                                                                                                                                                                                                             |
| `library.tabs`                  | The library's tab switcher.                                                                                                                                                                                                                                                                                                              |
| `library.search`                | The library's search box.                                                                                                                                                                                                                                                                                                                |
| `library.toolbar.filter`        | The filter button (the funnel) that opens the tile size, sort and group menu.                                                                                                                                                                                                                                                            |
| `library.toolbar.arrange`       | The rearrange button, in the folders view.                                                                                                                                                                                                                                                                                               |
| `library.toolbar.create`        | The create button at the right of the toolbar.                                                                                                                                                                                                                                                                                           |
| `library.group`                 | One group in the grouped library view. It is a `collapsable`.                                                                                                                                                                                                                                                                            |
| `library.grid`                  | A grid of tiles: the library's own, each group's and an open folder's. See [Tiles](#tiles).                                                                                                                                                                                                                                              |
| `library.empty`                 | The message shown when the library has nothing in it.                                                                                                                                                                                                                                                                                    |
| `library.empty.text`            | The words of that message.                                                                                                                                                                                                                                                                                                               |
| `library.tile`                  | An instance, server or folder tile.                                                                                                                                                                                                                                                                                                      |
| `library.tile.selected`         | Added to a tile that is selected (Ctrl or Cmd click, or drag-select).                                                                                                                                                                                                                                                                    |
| `library.tile.running`          | Added to a tile while its game or server is running.                                                                                                                                                                                                                                                                                     |
| `library.tile.busy`             | Added to a server tile while it starts or stops.                                                                                                                                                                                                                                                                                         |
| `library.tile.loading`          | Added to a tile while it installs or waits in the queue.                                                                                                                                                                                                                                                                                 |
| `library.tile.favorite`         | Added to a favourite instance or server.                                                                                                                                                                                                                                                                                                 |
| `library.tile.fresh`            | Added to a new instance, one that shows the NEW badge.                                                                                                                                                                                                                                                                                   |
| `library.tile.frame`            | The rounded frame round the picture: 2 px of padding, and it clips what's inside. Its rounding is `--gdl-tile-radius`.                                                                                                                                                                                                                   |
| `library.tile.outline`          | The outline drawn round a selected tile, an SVG. On an instance or server tile it fills `library.tile.frame`; on a folder it sits outside the frame at the picture's size plus 2 px round it. Its corners follow `--gdl-tile-radius`; `stroke` and `stroke-width` on the hook restyle it. With reduced motion it is drawn whole at once. |
| `library.tile.image`            | The box that holds the picture, the layers over it and, in `overlay` style, the name and buttons. It clips them.                                                                                                                                                                                                                         |
| `library.tile.picture`          | The picture itself, inside `library.tile.image`. It zooms and blurs when you hover the tile, and turns grey while it installs. See [Tiles](#tiles).                                                                                                                                                                                      |
| `library.tile.glow`             | The coloured ring around the picture while the tile runs, installs or is busy.                                                                                                                                                                                                                                                           |
| `library.tile.dim`              | The dark layer over the picture on hover.                                                                                                                                                                                                                                                                                                |
| `library.tile.shade`            | The dark gradient behind the name and details on the picture (`tileStyle` `overlay`).                                                                                                                                                                                                                                                    |
| `library.tile.checkbox`         | The selection checkbox in the top-left corner, on hover or when selected.                                                                                                                                                                                                                                                                |
| `library.tile.platform`         | The CurseForge or Modrinth icon on a modpack's tile.                                                                                                                                                                                                                                                                                     |
| `library.tile.star`             | The star before a favourite server's name.                                                                                                                                                                                                                                                                                               |
| `library.tile.play`             | The play button on a tile.                                                                                                                                                                                                                                                                                                               |
| `library.tile.play.running`     | Added to the play button while the game is running.                                                                                                                                                                                                                                                                                      |
| `library.tile.play.busy`        | Added to the play button while the instance is busy (installing, for example).                                                                                                                                                                                                                                                           |
| `library.tile.name`             | The name on a tile.                                                                                                                                                                                                                                                                                                                      |
| `library.tile.meta`             | The version and loader line on a tile.                                                                                                                                                                                                                                                                                                   |
| `library.tile.caption`          | The block under the picture with `tileStyle` `caption`.                                                                                                                                                                                                                                                                                  |
| `library.tile.new`              | The NEW badge on a tile.                                                                                                                                                                                                                                                                                                                 |
| `library.tile.veil`             | The blur over the picture while the tile installs, waits in the queue or is deleted.                                                                                                                                                                                                                                                     |
| `library.tile.fade`             | The two dark gradients over the picture at the same times (two elements).                                                                                                                                                                                                                                                                |
| `library.tile.progress`         | The block in the middle of an installing tile: the percentage and the task names.                                                                                                                                                                                                                                                        |
| `library.tile.progress.percent` | The big percentage.                                                                                                                                                                                                                                                                                                                      |
| `library.tile.progress.task`    | One task's name under it.                                                                                                                                                                                                                                                                                                                |
| `library.tile.spinner`          | The spinner and its words while the tile waits in the queue or is deleted.                                                                                                                                                                                                                                                               |
| `library.tile.bar`              | The strip at the bottom of an installing tile, with the progress bar and the size done.                                                                                                                                                                                                                                                  |
| `library.tile.bar.track`        | The progress bar's track.                                                                                                                                                                                                                                                                                                                |
| `library.tile.bar.fill`         | The part of the track that is done.                                                                                                                                                                                                                                                                                                      |
| `library.tile.bar.text`         | The size done, like `12.5 / 40.0 MB`.                                                                                                                                                                                                                                                                                                    |
| `library.folder`                | Added to a folder tile, next to `library.tile`.                                                                                                                                                                                                                                                                                          |
| `library.dock`                  | The favourites dock at the bottom left of the library.                                                                                                                                                                                                                                                                                   |
| `library.dock.star`             | The star at the start of the dock.                                                                                                                                                                                                                                                                                                       |
| `library.dock.item`             | One favourite in the dock, shown on hover.                                                                                                                                                                                                                                                                                               |
| `library.dock.item.running`     | Added to a favourite while it runs.                                                                                                                                                                                                                                                                                                      |
| `library.dock.item.play`        | The play layer over a favourite, on hover.                                                                                                                                                                                                                                                                                               |

### Instance and server pages

The page of one instance or one server. The two pages share their layout, so most hooks come in pairs (`instance.cover` and `server.cover`).

| Hook                            | Where it is                                                                                                                                               |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `instance.header`               | The cover header of an instance page.                                                                                                                     |
| `instance.header.fade`          | The two fade layers over the instance cover (two elements).                                                                                               |
| `instance.cover`                | The cover picture in the header.                                                                                                                          |
| `instance.back`                 | The round back button at the top left of the cover.                                                                                                       |
| `instance.corner`               | The round buttons at the top right of the cover: more, share and favourite.                                                                               |
| `instance.menu`                 | The more button (…) that opens the instance's menu.                                                                                                       |
| `instance.share`                | The share button.                                                                                                                                         |
| `instance.favorite`             | The favourite button (the star).                                                                                                                          |
| `instance.favorite.active`      | Added to the favourite button when the instance is a favourite.                                                                                           |
| `instance.icon`                 | The small square picture next to the name.                                                                                                                |
| `instance.info`                 | The block with the name, the details line and the play button.                                                                                            |
| `instance.meta`                 | The details line under the name: loader, version, time played and authors.                                                                                |
| `instance.actions`              | The place of the play button in the header.                                                                                                               |
| `instance.play`                 | The big play button. It is also a `button`.                                                                                                               |
| `instance.tabs`                 | The tabs of an instance page.                                                                                                                             |
| `instance.tabs.back`            | The back button that slides into the tab bar once it sticks to the top.                                                                                   |
| `instance.tabs.play`            | The small play button that slides in at the right of the tab bar once it sticks.                                                                          |
| `instance.content`              | The area under the tab bar that holds the open tab.                                                                                                       |
| `instance.overview`             | The Overview tab.                                                                                                                                         |
| `instance.overview.card`        | One of the cards on the Overview tab: Minecraft version, modloader, mods, time played and more.                                                           |
| `instance.overview.card.icon`   | A card's picture.                                                                                                                                         |
| `instance.overview.card.value`  | A card's big value, like `1.21.1`.                                                                                                                        |
| `instance.overview.card.label`  | A card's label, like MINECRAFT VERSION.                                                                                                                   |
| `instance.overview.modpack`     | The modpack card, on a modpack's instance.                                                                                                                |
| `instance.overview.notes`       | The notes card, when the instance has notes.                                                                                                              |
| `instance.overview.notes.label` | The notes card's NOTES label.                                                                                                                             |
| `instance.overview.notes.text`  | The notes themselves.                                                                                                                                     |
| `addons.filters`                | The bar at the top of the Addons tab (instances and servers): search, platform, buttons and the type chips.                                               |
| `addons.table`                  | The list of installed add-ons.                                                                                                                            |
| `addons.table.header`           | Its header row with the column names.                                                                                                                     |
| `addons.table.header.cell`      | One column name in that row.                                                                                                                              |
| `addons.row`                    | One add-on in the list.                                                                                                                                   |
| `addons.row.selected`           | Added to the selected rows.                                                                                                                               |
| `addons.row.targeted`           | Added for a moment to the rows a right-click menu or a drag-select is about to act on, including rows a drag is about to deselect.                        |
| `addons.row.deselecting`        | Added with `addons.row.targeted` while a drag is about to deselect the row (the app shows it in red); without it, a targeted row is about to be selected. |
| `addons.row.disabled`           | Added to a disabled add-on.                                                                                                                               |
| `addons.cell`                   | One cell of a row.                                                                                                                                        |
| `addons.empty`                  | The message shown when nothing is installed.                                                                                                              |
| `logs`                          | The Logs tab of an instance.                                                                                                                              |
| `logs.sidebar`                  | The list of game sessions at its left.                                                                                                                    |
| `logs.session`                  | One session in that list.                                                                                                                                 |
| `logs.session.selected`         | Added to the session on show.                                                                                                                             |
| `logs.content`                  | The dark box the log lines are in.                                                                                                                        |
| `logs.line`                     | One log line.                                                                                                                                             |
| `logs.line.comfortable`         | Added to every line when Logs Density is Comfortable.                                                                                                     |
| `logs.line.compact`             | Added to every line when Logs Density is Compact.                                                                                                         |
| `logs.line.number`              | A line's number, at its start.                                                                                                                            |
| `logs.line.time`                | A line's date and time.                                                                                                                                   |
| `logs.line.source`              | Where a line came from, like `[GAME]`.                                                                                                                    |
| `logs.line.level`               | A line's level, like `[INFO]` or `[ERROR]`.                                                                                                               |
| `logs.line.logger`              | The logger that wrote the line.                                                                                                                           |
| `logs.line.thread`              | The thread that wrote the line.                                                                                                                           |
| `logs.line.message`             | A line's message.                                                                                                                                         |
| `logs.live`                     | The LIVE label while the game runs.                                                                                                                       |
| `logs.empty`                    | The message shown when a session has no lines.                                                                                                            |
| `server.header`                 | The cover header of a server page.                                                                                                                        |
| `server.header.fade`            | The two fade layers over the server cover (two elements).                                                                                                 |
| `server.cover`                  | The cover picture in the header.                                                                                                                          |
| `server.back`                   | The round back button at the top left of the cover.                                                                                                       |
| `server.corner`                 | The round buttons at the top right of the cover, when the page has any (it has none today).                                                               |
| `server.icon`                   | The small square picture next to the name.                                                                                                                |
| `server.info`                   | The block with the name, the details line and the buttons.                                                                                                |
| `server.meta`                   | The details line under the name: type, version, port and status.                                                                                          |
| `server.status`                 | The status pill in the details line.                                                                                                                      |
| `server.status.running`         | Added while the server runs.                                                                                                                              |
| `server.status.starting`        | Added while it starts.                                                                                                                                    |
| `server.status.stopping`        | Added while it stops.                                                                                                                                     |
| `server.favorite`               | The favourite button (the star).                                                                                                                          |
| `server.favorite.active`        | Added when the server is a favourite.                                                                                                                     |
| `server.actions`                | The buttons in the header: favourite, repair and start.                                                                                                   |
| `server.play`                   | The big start button. It is also a `button`.                                                                                                              |
| `server.tabs`                   | The tabs of a server page.                                                                                                                                |
| `server.tabs.back`              | The back button that slides into the tab bar once it sticks to the top.                                                                                   |
| `server.tabs.play`              | The small start button that slides in at the right of the tab bar.                                                                                        |
| `server.content`                | The area under the tab bar that holds the open tab.                                                                                                       |
| `server.console`                | The dark box of the server console.                                                                                                                       |
| `server.console.line`           | One console line.                                                                                                                                         |

### Search and add-on pages

The Search page with its filter sidebar and results, and the page of one mod or modpack.

| Hook                        | Where it is                                                                                                                                                                                                 |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `search.context`            | The box at the top of the Search sidebar showing the instance or server.                                                                                                                                    |
| `search.context.fade`       | The sideways fade layer in that box.                                                                                                                                                                        |
| `search.context.shade`      | The bottom fade layer in that box.                                                                                                                                                                          |
| `search.sidebar.fade`       | The fade at the bottom of the Search filter sidebar.                                                                                                                                                        |
| `search.sidebar`            | The filter sidebar, docked or flying out (two elements, the docked one and the fly-out). It is also a `page.body`.                                                                                          |
| `search.sidebar.header`     | Its top row: Filters, clear all, the view switch and the dock button.                                                                                                                                       |
| `search.sidebar.title`      | The word "Filters" in that row.                                                                                                                                                                             |
| `search.sidebar.collapsed`  | The strip of filter icons shown when the sidebar is undocked.                                                                                                                                               |
| `search.sidebar.flyout`     | The panel the undocked sidebar flies out in on hover, over the results. It has the panel's colour of its own, so a theme that clears `page.body` still covers the results there; give it your panel's look. |
| `search.filter`             | One filter section: platform, categories, loaders, versions, environment or sort. Each holds a `collapsable`.                                                                                               |
| `search.filter.option`      | One choice with a checkbox in a filter.                                                                                                                                                                     |
| `category-icon`             | A category's picture: in the Categories filter, and on the category chips of results and filters.                                                                                                           |
| `search.results`            | The results area, in the list or the grid view.                                                                                                                                                             |
| `search.result`             | One result.                                                                                                                                                                                                 |
| `search.result.grid`        | Added to a result in the grid view.                                                                                                                                                                         |
| `search.result.installed`   | Added to a result that is already installed where you're adding it.                                                                                                                                         |
| `search.result.image`       | A result's picture: the icon in the list, the whole card in the grid.                                                                                                                                       |
| `search.result.backdrop`    | The blurred picture behind a result in the list.                                                                                                                                                            |
| `search.result.title`       | A result's name.                                                                                                                                                                                            |
| `search.result.description` | A result's summary.                                                                                                                                                                                         |
| `search.result.stats`       | The download count and platform icon at the right of a list row.                                                                                                                                            |
| `search.result.action`      | The install button at the right of a list row.                                                                                                                                                              |
| `search.result.panel`       | The glass panel at the bottom of a grid card.                                                                                                                                                               |
| `search.empty`              | The message shown when nothing matches. Results load as you scroll; there are no pages.                                                                                                                     |
| `addon-page`                | The page of one mod, modpack or other add-on.                                                                                                                                                               |
| `addon-page.header`         | Its cover header.                                                                                                                                                                                           |
| `addon-page.header.fade`    | The two fade layers in that header (two elements).                                                                                                                                                          |
| `addon-page.cover`          | The cover picture.                                                                                                                                                                                          |
| `addon-page.back`           | The back button over the cover.                                                                                                                                                                             |
| `addon-page.icon`           | The small square picture next to the name.                                                                                                                                                                  |
| `addon-page.actions`        | The install buttons under the name.                                                                                                                                                                         |
| `addon-page.tabs`           | The sticky tab bar under the header.                                                                                                                                                                        |

### Settings

Settings, and the Settings tabs of instances and servers, which use the same rows.

| Hook                       | Where it is                                                                                                                                                                                                                                               |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `settings.nav`             | The sticky tab bar across the top of Settings.                                                                                                                                                                                                            |
| `settings.nav.item`        | One page in that bar. It is also a `tab`.                                                                                                                                                                                                                 |
| `settings.rows`            | The list that holds a page's rows. It draws the lines between them, on each row's top border: a plain `settings.row` rule replaces them. On Accounts it also holds the accounts table, whose line is on the table: `[data-gdl~="settings.rows"] > table`. |
| `settings.row`             | One row in Settings.                                                                                                                                                                                                                                      |
| `settings.row.title`       | A row's title. It is also a `section-title`.                                                                                                                                                                                                              |
| `settings.row.description` | The line under a row's title that explains it.                                                                                                                                                                                                            |
| `settings.row.control`     | The switch, select or buttons at the right of a row.                                                                                                                                                                                                      |
| `settings.about`           | The card at the bottom of General with the logo, Check for Updates, Restart App and the version. It is also a `settings.row`.                                                                                                                             |

### News

The News page.

| Hook                        | Where it is                                                                                        |
| --------------------------- | -------------------------------------------------------------------------------------------------- |
| `news`                      | The whole News page.                                                                               |
| `news.tabs`                 | The sticky tab bar at the top of News (News and Patch Notes). It is also a `page.body`.            |
| `news.slideshow`            | The big slideshow of stories at the top of the News tab.                                           |
| `news.slideshow.prev`       | The arrow to the previous story, on hover.                                                         |
| `news.slideshow.next`       | The arrow to the next story, on hover.                                                             |
| `news.slideshow.dots`       | The row of dots under the slides.                                                                  |
| `news.slideshow.dot`        | One dot.                                                                                           |
| `news.slideshow.dot.active` | Added to the dot of the story on show.                                                             |
| `news.slide`                | One story in the slideshow, with its picture as the background.                                    |
| `news.slide.veil`           | The dark layer over the picture. A fixed dark colour, whatever the palette.                        |
| `news.slide.title`          | A story's title: your `lightSlate-50`, or white when that is darker than 70%.                      |
| `news.slide.text`           | A story's summary: your `lightSlate-200`, or a light grey when `lightSlate-50` is darker than 70%. |
| `news.slide.date`           | A story's date, like the summary.                                                                  |
| `news.heading`              | The "Minecraft News" heading above the cards.                                                      |
| `news.grid`                 | The grid of news cards.                                                                            |
| `news.card`                 | One news card.                                                                                     |
| `news.card.image`           | A card's picture.                                                                                  |
| `news.card.title`           | A card's title.                                                                                    |
| `news.card.text`            | A card's summary.                                                                                  |
| `news.card.date`            | A card's date.                                                                                     |
| `news.patch`                | One release card on the Patch Notes tab.                                                           |

### Dialogs

Every dialog uses the same frame.

| Hook                                | Where it is                                                                                                                                                                                                                                                 |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `modal`                             | A dialog.                                                                                                                                                                                                                                                   |
| `modal.header`                      | A dialog's header.                                                                                                                                                                                                                                          |
| `modal.title`                       | A dialog's title.                                                                                                                                                                                                                                           |
| `modal.close`                       | The close button (×) in the header.                                                                                                                                                                                                                         |
| `modal.body`                        | Everything under the header.                                                                                                                                                                                                                                |
| `modal.footer`                      | The row of buttons at the bottom: confirm dialogs, creating an instance or server, installing or removing a theme.                                                                                                                                          |
| `modal.backdrop`                    | The dark layer behind a dialog, over the page: `darkSlate-900` at 95%.                                                                                                                                                                                      |
| `modal.column-backdrop`             | The same dark layer over the ad column's backdrop while a dialog is open, in the same colour as `modal.backdrop`; restyle the two together. It lies under the ad, which stays on top: style only its background. Nothing may cover, hide or imitate the ad. |
| `season-dialog`                     | The dialog that offers a seasonal theme.                                                                                                                                                                                                                    |
| `season-dialog.art`                 | The picture in that dialog.                                                                                                                                                                                                                                 |
| `season-dialog.name`                | The theme's name in that dialog.                                                                                                                                                                                                                            |
| `instance-creation.divider`         | A titled divider in the New Instance dialog: About, Minecraft version and Modloader.                                                                                                                                                                        |
| `instance-creation.divider.line`    | One of the two lines either side of a divider's title.                                                                                                                                                                                                      |
| `instance-creation.icon`            | The picture picker for the new instance's icon.                                                                                                                                                                                                             |
| `instance-creation.loader`          | One modloader card (Vanilla, Forge, NeoForge, Fabric, Quilt).                                                                                                                                                                                               |
| `instance-creation.loader.selected` | Added to the card that is chosen.                                                                                                                                                                                                                           |

### Controls

Buttons, fields and menus, wherever they are. A control in a hooked area has both, so `[data-gdl~="library.toolbar"] [data-gdl~="button"]` picks the toolbar's buttons.

| Hook                         | Where it is                                                                                                                                                                                                                                                                                 |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `button`                     | Every button.                                                                                                                                                                                                                                                                               |
| `button.primary`             | The main button style.                                                                                                                                                                                                                                                                      |
| `button.tinted`              | Added to buttons with their own colour, like red or green ones.                                                                                                                                                                                                                             |
| `button.disabled`            | Added to a disabled button.                                                                                                                                                                                                                                                                 |
| `button.loading`             | Added to a button while it shows its spinner or progress.                                                                                                                                                                                                                                   |
| `button.secondary`           | The secondary button style.                                                                                                                                                                                                                                                                 |
| `button.glow`                | The glowing button style.                                                                                                                                                                                                                                                                   |
| `button.transparent`         | The see-through button style, like the round buttons on a cover.                                                                                                                                                                                                                            |
| `button.glass`               | The glass button style.                                                                                                                                                                                                                                                                     |
| `button.text`                | The plain text button style.                                                                                                                                                                                                                                                                |
| `badge`                      | A small label.                                                                                                                                                                                                                                                                              |
| `badge.default`              | The default (filled) badge colour.                                                                                                                                                                                                                                                          |
| `platform-logo`              | A CurseForge or Modrinth logo, with `platform-logo.curseforge` or `platform-logo.modrinth`. The CurseForge one is drawn in `color`: white on a dark palette and the text colour on a light one, by default; set `color` to recolour it. The Modrinth one is a picture.                      |
| `input`                      | A text input, with its frame.                                                                                                                                                                                                                                                               |
| `input.disabled`             | Added to a disabled input.                                                                                                                                                                                                                                                                  |
| `input.invalid`              | Added to an input with an error.                                                                                                                                                                                                                                                            |
| `input.field`                | The field inside an input. It has its own solid background, so clear it to show the frame's.                                                                                                                                                                                                |
| `input.icon`                 | The icon at the right of an input, like the search magnifier.                                                                                                                                                                                                                               |
| `input.error`                | The error message under an input.                                                                                                                                                                                                                                                           |
| `input.suggestions`          | The list of suggestions some inputs open.                                                                                                                                                                                                                                                   |
| `input.suggestion`           | One suggestion in that list.                                                                                                                                                                                                                                                                |
| `select`                     | A drop-down select: the box you click.                                                                                                                                                                                                                                                      |
| `select.value`               | The chosen value shown in the box.                                                                                                                                                                                                                                                          |
| `select.icon`                | The arrows at the right of the box.                                                                                                                                                                                                                                                         |
| `select.item`                | One choice in the list a select opens (the list is a `dropdown`).                                                                                                                                                                                                                           |
| `select.item.selected`       | Added to the chosen one.                                                                                                                                                                                                                                                                    |
| `select.item.disabled`       | Added to a choice that can't be picked.                                                                                                                                                                                                                                                     |
| `switch`                     | A switch.                                                                                                                                                                                                                                                                                   |
| `switch.checked`             | Added to a switch that is on, not while it's loading or indeterminate.                                                                                                                                                                                                                      |
| `switch.loading`             | Added while the switch waits on a change; the knob sits in the middle.                                                                                                                                                                                                                      |
| `switch.indeterminate`       | Added when the switch is neither on nor off; the knob sits in the middle.                                                                                                                                                                                                                   |
| `switch.disabled`            | Added to a disabled switch.                                                                                                                                                                                                                                                                 |
| `switch.track`               | The switch's track.                                                                                                                                                                                                                                                                         |
| `switch.thumb`               | The knob that slides.                                                                                                                                                                                                                                                                       |
| `checkbox`                   | A checkbox, with its label.                                                                                                                                                                                                                                                                 |
| `checkbox.checked`           | Added when it's ticked.                                                                                                                                                                                                                                                                     |
| `checkbox.indeterminate`     | Added when it shows a dash instead: part of what it stands for is ticked.                                                                                                                                                                                                                   |
| `checkbox.disabled`          | Added when it's disabled.                                                                                                                                                                                                                                                                   |
| `checkbox.box`               | The box.                                                                                                                                                                                                                                                                                    |
| `checkbox.mark`              | The tick inside the box.                                                                                                                                                                                                                                                                    |
| `checkbox.dash`              | The dash a checkbox shows when part of what it stands for is ticked.                                                                                                                                                                                                                        |
| `radio`                      | A radio choice, with its label.                                                                                                                                                                                                                                                             |
| `radio.checked`              | Added to the chosen one.                                                                                                                                                                                                                                                                    |
| `radio.dot`                  | The round mark.                                                                                                                                                                                                                                                                             |
| `slider`                     | A slider, like the Java memory slider.                                                                                                                                                                                                                                                      |
| `slider.track`               | Its track.                                                                                                                                                                                                                                                                                  |
| `slider.fill`                | The part of the track up to the knob.                                                                                                                                                                                                                                                       |
| `slider.thumb`               | The knob.                                                                                                                                                                                                                                                                                   |
| `slider.mark`                | A tick on the track.                                                                                                                                                                                                                                                                        |
| `slider.label`               | The words under a tick, like `8 GB`. A label that would run into the one before it is hidden.                                                                                                                                                                                               |
| `slider.tooltip`             | The value shown over the knob while you drag or point at it.                                                                                                                                                                                                                                |
| `collapsable`                | A section that folds open and shut: a library group, a Search filter section, the Customize section on Appearance, and some sections in Settings, Accounts, the share dialog and the GDLauncher account sign-in.                                                                            |
| `collapsable.open`           | Added while it's open.                                                                                                                                                                                                                                                                      |
| `collapsable.header`         | Its header row, which you click.                                                                                                                                                                                                                                                            |
| `collapsable.chevron`        | The arrow in the header. It turns when the section opens.                                                                                                                                                                                                                                   |
| `collapsable.title`          | The title in the header.                                                                                                                                                                                                                                                                    |
| `collapsable.count`          | The count of active choices in the header (Search filters).                                                                                                                                                                                                                                 |
| `collapsable.clear`          | The × that clears those choices.                                                                                                                                                                                                                                                            |
| `collapsable.body`           | What folds.                                                                                                                                                                                                                                                                                 |
| `tabs`                       | A tab bar.                                                                                                                                                                                                                                                                                  |
| `tabs.indicator`             | The moving highlight in a tab bar.                                                                                                                                                                                                                                                          |
| `tab`                        | One tab.                                                                                                                                                                                                                                                                                    |
| `tab.active`                 | The selected tab.                                                                                                                                                                                                                                                                           |
| `tab.icon`                   | The icon in a tab: the library, instance, server, add-on, Settings, News and nav tabs.                                                                                                                                                                                                      |
| `tooltip`                    | A tooltip.                                                                                                                                                                                                                                                                                  |
| `context-menu`               | A right-click menu.                                                                                                                                                                                                                                                                         |
| `context-menu.item`          | One item in it.                                                                                                                                                                                                                                                                             |
| `context-menu.item.disabled` | Added to an item that can't be used.                                                                                                                                                                                                                                                        |
| `context-menu.label`         | A heading inside the menu. The add-ons menu's "N selected" heading is a disabled item that also carries this hook, so your `context-menu.item` and `context-menu.item.disabled` rules reach it too; `[data-gdl~="context-menu.item"][data-gdl~="context-menu.label"]` styles it on its own. |
| `context-menu.separator`     | A line between groups of items.                                                                                                                                                                                                                                                             |
| `context-menu.submenu`       | A menu that opens from an item. It doesn't also have `context-menu`.                                                                                                                                                                                                                        |
| `dropdown`                   | A drop-down menu, a select's list or a popover. A menu that opens from an item has only `dropdown.submenu`.                                                                                                                                                                                 |
| `dropdown.item`              | One item in a drop-down menu.                                                                                                                                                                                                                                                               |
| `dropdown.item.checked`      | Added to the chosen or ticked item.                                                                                                                                                                                                                                                         |
| `dropdown.item.disabled`     | Added to an item that can't be used.                                                                                                                                                                                                                                                        |
| `dropdown.label`             | A heading inside the menu.                                                                                                                                                                                                                                                                  |
| `dropdown.separator`         | A line between groups of items.                                                                                                                                                                                                                                                             |
| `dropdown.submenu`           | A menu that opens from an item, like the tile size choices in the library's filter menu.                                                                                                                                                                                                    |

### Toasts and layers

| Hook                | Where it is                                                               |
| ------------------- | ------------------------------------------------------------------------- |
| `toast`             | A toast. Its colours are set inline, so changing them needs `!important`. |
| `toast.success`     | Added to a success toast.                                                 |
| `toast.error`       | Added to an error toast.                                                  |
| `toast.warning`     | Added to a warning toast.                                                 |
| `toast.info`        | Added to an info toast.                                                   |
| `toast.loading`     | Added to a toast that is waiting on something.                            |
| `toast.icon`        | A toast's icon.                                                           |
| `toast.title`       | A toast's title.                                                          |
| `toast.description` | A toast's second line.                                                    |
| `toast.action`      | A toast's action button. Also styled inline.                              |
| `toast.cancel`      | A toast's cancel button. Also styled inline.                              |
| `toast.close`       | The × that closes a toast.                                                |
| `toast-region`      | The area toasts appear in.                                                |
| `background`        | The theme's background layer.                                             |

## Identity tokens

Some parts come in many kinds that look alike in the page: dialogs, Settings tabs, Search filters. To tell them apart without counting siblings (`:nth-child` breaks when we add one, and counts moving highlights too), each carries an identity token next to its base hook: **`<base>.<id>`**, with the id in kebab-case, like `modal.instance-creation`. Unlike a state, an identity never comes and goes while the element is on the page.

```css
/* Only the New Instance dialog. */
[data-gdl~="modal.instance-creation"] [data-gdl~="modal.header"] {
  border-bottom: 2px solid rgb(var(--primary-500));
}

/* Each Settings tab's icon in its own colour. */
[data-gdl~="settings.nav.item.java"] [data-gdl~="tab.icon"] {
  color: rgb(230 140 60);
}
```

### `settings.nav.item.<id>`

One per Settings page, on its tab:

```
general, accounts, language, appearance, java, custom-commands, privacy,
runtime-path
```

### `search.filter.<id>`

One per filter section in the Search sidebar:

```
platform, categories, modloaders, game-versions, environment, sort
```

### `modal.<id>`

Every dialog, by its name in the launcher. The ones people meet most are `instance-creation` (New Instance), `server-creation`, `theme-install`, `theme-remove`, `confirm-instance-deletion`, `export-instance`, `share-instance` and `seasonal-theme`. The full list:

```
privacy-statement, terms-and-conditions, add-managed-java, delete-java-confirm,
remove-unused-javas-confirm, delete-java-profile-confirm, java-issue,
shader-loader-setup, instance-creation, export-instance, modpack-version-update,
unlock-confirmation, unpair-confirmation, notification,
confirm-instance-deletion, repair-modpack, kept-pack-files,
confirm-batch-instance-deletion, confirm-server-deletion,
confirm-batch-server-deletion, server-eula-acceptance, recovered-server-process,
force-stop-server, confirm-batch-folder-deletion, confirm-batch-mixed-deletion,
confirm-change-runtime-path, runtime-move-notice, runtime-path-unavailable,
on-boarding, why-are-ads-needed, mods-updater, java-profile-creation,
window-close-warning, changelogs, confirm-gdl-account-deletion,
confirm-ms-with-gdl-account-removal, account-expired,
change-gdl-account-recovery-email, edit-gdl-profile,
change-gdl-account-display-name, mod-details, platform-selection,
duplicated-mods-resolution, beta-prompt, seasonal-theme, theme-remove,
theme-install, share-instance, my-shares, edit-share, share-preview, report,
account-banned, requires-gdl-account, insufficient-memory,
confirm-addon-deletion, confirm-world-deletion, confirm-choice, server-creation,
server-rename, cache-cleanup
```

### `platform-logo.<id>`

On a CurseForge or Modrinth logo:

```
curseforge, modrinth
```

## Decorations

A decoration is an image the launcher attaches to a part of the page, like the snow on a tile or the hat on the logo. It is drawn as a CSS background of an element, never inlined, so scripts in an SVG never run. Decorations ignore the mouse, and they're drawn over the part they're on, text included, so keep them clear of what people need to read.

### Anchors

The `anchor` names where the image goes. Only these 14 work; the other hooks are for CSS. `ad-column-background` and `button` are not anchors and are refused with a warning.

| Anchor            | What it decorates                                          |
| ----------------- | ---------------------------------------------------------- |
| `content`         | The whole content column.                                  |
| `nav`             | The top bar, or the rail.                                  |
| `nav.logo`        | The logo.                                                  |
| `panel`           | The panel of a page.                                       |
| `library.tile`    | Each tile. `pick` `random` is stable per tile.             |
| `modal`           | Each dialog.                                               |
| `season-dialog`   | The season dialog.                                         |
| `section-title`   | Each section title (Settings sections and library groups). |
| `library.group`   | Each group in the grouped library.                         |
| `library.toolbar` | The library's toolbar.                                     |
| `instance.header` | An instance page's cover header.                           |
| `server.header`   | A server page's cover header.                              |
| `settings.nav`    | The tab bar across the top of Settings.                    |
| `page-title`      | Settings page titles.                                      |

### Where it is cut off

A decoration is cut off at the edges of what holds it:

- **Sideways**, every decoration in the page is cut by the content column, and every one in a dialog at the same edge, so nothing reaches the ad column.
- **Up and down**, by the panel (`panel`), the header (`instance.header`, `server.header`) or the dialog (`modal`) that holds it. On `panel`, you see 16 px above the panel (`y` down to `-16`); the frame cuts what's beyond.
- **`nav`** and **`nav.logo`** are cut exactly at the nav's own edges, so the logo's hat can't run down into the page or the ads.
- `season-dialog` isn't cut by its own box, so a cap can hang over its edge.

### Edge, offsets and size

`edge` says where the image is attached, `x` and `y` move it, and `width` and `height` size it. Lengths are px numbers or `%` of the anchor.

| Edge                         | The image             | Size you give                                  |
| ---------------------------- | --------------------- | ---------------------------------------------- |
| `top`, `bottom`              | Runs along that edge. | `height` (required). `width` defaults to 100%. |
| `left`, `right`              | Runs along that edge. | `width` (required). `height` defaults to 100%. |
| `top-left` and the 3 corners | Sits in that corner.  | Both `width` and `height` (required).          |

`x` is positive to the right and `y` is positive downward, on every edge. So `"y": -16` on a `top` edge moves the image up so it hangs over the edge. Edges and offsets are measured from inside the anchor's border, so giving the anchor (say `panel`) a 1 px border moves its decorations 1 px in: use `-17` instead of `-16` to keep them where they were. Percentages are of the anchor's own size, so a decoration on `library.tile` with `"width": "106%"` scales with the tile. A tile with a caption is taller, and `%` heights follow.

### Images, `pick` and `repeat`

- `images` takes image files only (no video, sound or font). Use SVG or WebP for flat art.
- `pick` chooses which image each item shows. `first` always uses the first. `random` picks one per item and stays the same for that item: a tile keeps the same snow cap, whether you scroll or restart. `cycle` hands them out one after another as items appear.
- `repeat` tiles the image: `x` repeats it sideways at the height of the strip, `y` repeats it down at the width of the strip. `none` stretches one image over the given size, so give it the same proportions as the picture.

## Effects, sounds and motion

### Motion

All motion and sound follow one state, `html[data-gdl-motion]`:

| State   | When                                                                                                              |
| ------- | ----------------------------------------------------------------------------------------------------------------- |
| `run`   | The normal state.                                                                                                 |
| `pause` | The window is hidden or minimised, or a game is running.                                                          |
| `off`   | Reduced motion: the launcher's own setting, or the system's "reduce motion". Until settings load it counts as on. |

| Thing                  | `run`    | `pause`                      | `off`                  |
| ---------------------- | -------- | ---------------------------- | ---------------------- |
| Snow                   | Falls.   | Stops, keeps its last frame. | Not drawn.             |
| String lights          | Twinkle. | Stop, keep their last frame. | Drawn once, still.     |
| Particles              | Move.    | Stop, keep their last frame. | Not drawn.             |
| Video `background.src` | Plays.   | Pauses.                      | Shows its first frame. |
| `background.motion`    | Drawn.   | Not drawn.                   | Not drawn.             |
| Sounds                 | Play.    | Silent.                      | Silent.                |

No effect, sound or video of a theme runs a timer or a loop outside `run`, not even to retry a file that failed to load. Your own CSS animations aren't managed: use the attribute to stop them.

```css
html:not([data-gdl-motion="run"]) [data-gdl~="library.tile"] {
  animation: none;
}
```

On top of motion, the Customize switches below the themes on the Appearance page turn things off for one person (Customize shows when the saved theme, built-in or installed, has sounds or effects, and starts closed). **Theme effects** is on by default; **Play theme sounds** is off by default. Effects, particles and string lights draw only when the effects switch is on, and sounds play only when the sounds switch is on.

### Effects

All effects are drawn on canvases inside `content`, so they never reach the ad column. They are capped for speed: at most 600 snowflakes and 150 particles however high the density, and the animation runs at a fixed, low frame rate. `density` is per window area, so a bigger window has more.

- **`snow`** falls in up to 3 depth layers (far ones small and faint, near ones large). `wind` bends the fall.
- **`stringLights`** hang from the nav's bottom edge across the content column, with the glow around each bulb.
- Effects and particles are drawn over the page, text included, so a dense one can make the page hard to read.
- **`particles`** are your pictures. The `preset` is the motion: `fall` drops them downward with a sway, `float` rises them slowly and fades them in and out, and `drift` slides them across. Each image is drawn once into a small bitmap, sized by `maxSize`, and reused. A broken or missing image is skipped with a warning. An **animated** sprite (GIF, WebP, APNG or an animated SVG) shows only its first frame.

### Sounds

- They play only when the person turns on **Play theme sounds**. It's off by default, so a first run is silent.
- The volume is fixed and low (30%). A theme chooses the files, never the level.
- A sound is cut at 4 seconds, however long the file is, so keep them short.
- The same event can't repeat too fast: `launch` at most every 2 s, `downloadComplete` every 1 s, `notification` and `error` every 0.5 s and `click` every 0.12 s.
- They're silent while motion isn't `run` (reduced motion, hidden window, a game running). A sound that's playing stops when that happens.
- `launch` plays when a game starts launching. `downloadComplete` plays when an instance or server install task (instance prepare, instance mod install, server install, server modpack install or server mod install) ends without failing, which includes one the person cancelled. Other downloads, such as Java, stay silent. `click` plays for primary buttons that aren't disabled, and nothing else; it gives way to any other event's sound, so a click that starts a launch or a toast doesn't overlap it. `notification` plays for every toast that isn't an error, and `error` for an error toast. A toast that says "loading" is silent.

## Safety and troubleshooting

### Safety

- **Nothing loads from the internet.** CSS, fonts, images, sounds and video all come from the theme's own files.
- **No code.** No scripts of any kind.
- **The ad can't be touched.** Ads fund the launcher, so they're protected by the way the launcher is built, not by rules a theme could get around. The ad lives in its own window layer on top of the page, placed by the launcher itself, and no theme, layout or setting can move, hide, shrink or cover it. The page always leaves the column free, in every layout, including `compact` and `rail`.
  - `ad-column-background` is a hook for the column's backdrop only: you can colour it or put a picture behind the ad. Its `background` is the column's colour, and the logo pattern on it is `ad-column-background.pattern`, which you can recolour with `color` or hide. Neither is an anchor.
  - The page's placeholders under the ad have no hook, and `--ad-column-width` is only there to read: setting it doesn't move the ad.
  - No decoration, effect or particle reaches into the column.
  - Don't try to cover, hide, fade, move or imitate the ad. A theme that does is a bug in the theme, and it still won't work.

### Troubleshooting

- **Warnings.** A theme that loads can still have warnings: an unknown key, an unknown anchor, a second effect of one type, an unknown colour name, an animated image, a `season` in an installed theme. They go to the developer tools console. Press `Alt+Shift+I` to open it and look for yellow lines. Each names the part of the manifest it is about.
- **A theme that won't load.** If a theme is invalid when it's turned on, or breaks after an edit, the launcher goes back to the default theme and shows a toast with one sentence from this list:

| The sentence                                    | What it means                                                                                                                          |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| The theme's files contain a mistake.            | Something in `manifest.json` or the CSS broke a rule in this guide: a bad value, a missing file, a refused URL, a file that's too big. |
| This theme needs a newer version of GDLauncher. | Its `themeApi` is higher than this launcher supports.                                                                                  |
| This theme isn't installed anymore.             | The folder is gone.                                                                                                                    |
| The theme's files couldn't be read.             | The launcher couldn't read the folder (a disk problem, not your files).                                                                |

- **The exact reason.** When you install a theme from a file, the dialog shows the sentence and, under it, the precise message (for example `theme.css line 12: only files inside the theme can be used, not "https://..."`). A theme that fails while it's already installed keeps the same message in the console. If the cause isn't clear, open the console first.
- **After a fix**, save the file. The launcher picks it up on its next check, and if you went back to the default, click your theme again.
- **Lost in a broken look?** Reset theme (`Ctrl+Alt+Shift+T`) always works.
