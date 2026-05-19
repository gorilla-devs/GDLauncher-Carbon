declare module 'astro:content' {
	interface Render {
		'.mdx': Promise<{
			Content: import('astro').MarkdownInstance<{}>['Content'];
			headings: import('astro').MarkdownHeading[];
			remarkPluginFrontmatter: Record<string, any>;
		}>;
	}
}

declare module 'astro:content' {
	interface Render {
		'.md': Promise<{
			Content: import('astro').MarkdownInstance<{}>['Content'];
			headings: import('astro').MarkdownHeading[];
			remarkPluginFrontmatter: Record<string, any>;
		}>;
	}
}

declare module 'astro:content' {
	type Flatten<T> = T extends { [K: string]: infer U } ? U : never;

	export type CollectionKey = keyof AnyEntryMap;
	export type CollectionEntry<C extends CollectionKey> = Flatten<AnyEntryMap[C]>;

	export type ContentCollectionKey = keyof ContentEntryMap;
	export type DataCollectionKey = keyof DataEntryMap;

	type AllValuesOf<T> = T extends any ? T[keyof T] : never;
	type ValidContentEntrySlug<C extends keyof ContentEntryMap> = AllValuesOf<
		ContentEntryMap[C]
	>['slug'];

	export function getEntryBySlug<
		C extends keyof ContentEntryMap,
		E extends ValidContentEntrySlug<C> | (string & {}),
	>(
		collection: C,
		// Note that this has to accept a regular string too, for SSR
		entrySlug: E
	): E extends ValidContentEntrySlug<C>
		? Promise<CollectionEntry<C>>
		: Promise<CollectionEntry<C> | undefined>;

	export function getDataEntryById<C extends keyof DataEntryMap, E extends keyof DataEntryMap[C]>(
		collection: C,
		entryId: E
	): Promise<CollectionEntry<C>>;

	export function getCollection<C extends keyof AnyEntryMap, E extends CollectionEntry<C>>(
		collection: C,
		filter?: (entry: CollectionEntry<C>) => entry is E
	): Promise<E[]>;
	export function getCollection<C extends keyof AnyEntryMap>(
		collection: C,
		filter?: (entry: CollectionEntry<C>) => unknown
	): Promise<CollectionEntry<C>[]>;

	export function getEntry<
		C extends keyof ContentEntryMap,
		E extends ValidContentEntrySlug<C> | (string & {}),
	>(entry: {
		collection: C;
		slug: E;
	}): E extends ValidContentEntrySlug<C>
		? Promise<CollectionEntry<C>>
		: Promise<CollectionEntry<C> | undefined>;
	export function getEntry<
		C extends keyof DataEntryMap,
		E extends keyof DataEntryMap[C] | (string & {}),
	>(entry: {
		collection: C;
		id: E;
	}): E extends keyof DataEntryMap[C]
		? Promise<DataEntryMap[C][E]>
		: Promise<CollectionEntry<C> | undefined>;
	export function getEntry<
		C extends keyof ContentEntryMap,
		E extends ValidContentEntrySlug<C> | (string & {}),
	>(
		collection: C,
		slug: E
	): E extends ValidContentEntrySlug<C>
		? Promise<CollectionEntry<C>>
		: Promise<CollectionEntry<C> | undefined>;
	export function getEntry<
		C extends keyof DataEntryMap,
		E extends keyof DataEntryMap[C] | (string & {}),
	>(
		collection: C,
		id: E
	): E extends keyof DataEntryMap[C]
		? Promise<DataEntryMap[C][E]>
		: Promise<CollectionEntry<C> | undefined>;

	/** Resolve an array of entry references from the same collection */
	export function getEntries<C extends keyof ContentEntryMap>(
		entries: {
			collection: C;
			slug: ValidContentEntrySlug<C>;
		}[]
	): Promise<CollectionEntry<C>[]>;
	export function getEntries<C extends keyof DataEntryMap>(
		entries: {
			collection: C;
			id: keyof DataEntryMap[C];
		}[]
	): Promise<CollectionEntry<C>[]>;

	export function reference<C extends keyof AnyEntryMap>(
		collection: C
	): import('astro/zod').ZodEffects<
		import('astro/zod').ZodString,
		C extends keyof ContentEntryMap
			? {
					collection: C;
					slug: ValidContentEntrySlug<C>;
				}
			: {
					collection: C;
					id: keyof DataEntryMap[C];
				}
	>;
	// Allow generic `string` to avoid excessive type errors in the config
	// if `dev` is not running to update as you edit.
	// Invalid collection names will be caught at build time.
	export function reference<C extends string>(
		collection: C
	): import('astro/zod').ZodEffects<import('astro/zod').ZodString, never>;

	type ReturnTypeOrOriginal<T> = T extends (...args: any[]) => infer R ? R : T;
	type InferEntrySchema<C extends keyof AnyEntryMap> = import('astro/zod').infer<
		ReturnTypeOrOriginal<Required<ContentConfig['collections'][C]>['schema']>
	>;

	type ContentEntryMap = {
		"blog": {
"best-modpacks-2026.md": {
	id: "best-modpacks-2026.md";
  slug: "best-modpacks-2026";
  body: string;
  collection: "blog";
  data: InferEntrySchema<"blog">
} & { render(): Render[".md"] };
"best-shaders-2026.md": {
	id: "best-shaders-2026.md";
  slug: "best-shaders-2026";
  body: string;
  collection: "blog";
  data: InferEntrySchema<"blog">
} & { render(): Render[".md"] };
"curseforge-partnership-announcement.md": {
	id: "curseforge-partnership-announcement.md";
  slug: "curseforge-partnership-announcement";
  body: string;
  collection: "blog";
  data: InferEntrySchema<"blog">
} & { render(): Render[".md"] };
"gdlauncher-carbon-out-now.md": {
	id: "gdlauncher-carbon-out-now.md";
  slug: "gdlauncher-carbon-out-now";
  body: string;
  collection: "blog";
  data: InferEntrySchema<"blog">
} & { render(): Render[".md"] };
};
"docs": {
"authentication-errors.md": {
	id: "authentication-errors.md";
  slug: "authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/authentication-errors.md": {
	id: "de/authentication-errors.md";
  slug: "de/authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/gdl-account-vs-microsoft-account.md": {
	id: "de/gdl-account-vs-microsoft-account.md";
  slug: "de/gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/gdlauncher-vs-gdlauncher-carbon.md": {
	id: "de/gdlauncher-vs-gdlauncher-carbon.md";
  slug: "de/gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/installation.md": {
	id: "de/installation.md";
  slug: "de/installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/instance-folder-anatomy.md": {
	id: "de/instance-folder-anatomy.md";
  slug: "de/instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/java-memory-and-gc.md": {
	id: "de/java-memory-and-gc.md";
  slug: "de/java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/locked-modpack-instances.md": {
	id: "de/locked-modpack-instances.md";
  slug: "de/locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/migration-from-legacy-gdlauncher.md": {
	id: "de/migration-from-legacy-gdlauncher.md";
  slug: "de/migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/mod-loaders-compared.md": {
	id: "de/mod-loaders-compared.md";
  slug: "de/mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/modpack-manifest-format.md": {
	id: "de/modpack-manifest-format.md";
  slug: "de/modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/offline-mode.md": {
	id: "de/offline-mode.md";
  slug: "de/offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/runtime-path-vs-app-data.md": {
	id: "de/runtime-path-vs-app-data.md";
  slug: "de/runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/save-format-compatibility.md": {
	id: "de/save-format-compatibility.md";
  slug: "de/save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"de/troubleshooting.md": {
	id: "de/troubleshooting.md";
  slug: "de/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/authentication-errors.md": {
	id: "es/authentication-errors.md";
  slug: "es/authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/gdl-account-vs-microsoft-account.md": {
	id: "es/gdl-account-vs-microsoft-account.md";
  slug: "es/gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/gdlauncher-vs-gdlauncher-carbon.md": {
	id: "es/gdlauncher-vs-gdlauncher-carbon.md";
  slug: "es/gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/installation.md": {
	id: "es/installation.md";
  slug: "es/installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/instance-folder-anatomy.md": {
	id: "es/instance-folder-anatomy.md";
  slug: "es/instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/java-memory-and-gc.md": {
	id: "es/java-memory-and-gc.md";
  slug: "es/java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/locked-modpack-instances.md": {
	id: "es/locked-modpack-instances.md";
  slug: "es/locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/migration-from-legacy-gdlauncher.md": {
	id: "es/migration-from-legacy-gdlauncher.md";
  slug: "es/migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/mod-loaders-compared.md": {
	id: "es/mod-loaders-compared.md";
  slug: "es/mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/modpack-manifest-format.md": {
	id: "es/modpack-manifest-format.md";
  slug: "es/modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/offline-mode.md": {
	id: "es/offline-mode.md";
  slug: "es/offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/runtime-path-vs-app-data.md": {
	id: "es/runtime-path-vs-app-data.md";
  slug: "es/runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/save-format-compatibility.md": {
	id: "es/save-format-compatibility.md";
  slug: "es/save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"es/troubleshooting.md": {
	id: "es/troubleshooting.md";
  slug: "es/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/authentication-errors.md": {
	id: "fr/authentication-errors.md";
  slug: "fr/authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/gdl-account-vs-microsoft-account.md": {
	id: "fr/gdl-account-vs-microsoft-account.md";
  slug: "fr/gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/gdlauncher-vs-gdlauncher-carbon.md": {
	id: "fr/gdlauncher-vs-gdlauncher-carbon.md";
  slug: "fr/gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/installation.md": {
	id: "fr/installation.md";
  slug: "fr/installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/instance-folder-anatomy.md": {
	id: "fr/instance-folder-anatomy.md";
  slug: "fr/instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/java-memory-and-gc.md": {
	id: "fr/java-memory-and-gc.md";
  slug: "fr/java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/locked-modpack-instances.md": {
	id: "fr/locked-modpack-instances.md";
  slug: "fr/locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/migration-from-legacy-gdlauncher.md": {
	id: "fr/migration-from-legacy-gdlauncher.md";
  slug: "fr/migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/mod-loaders-compared.md": {
	id: "fr/mod-loaders-compared.md";
  slug: "fr/mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/modpack-manifest-format.md": {
	id: "fr/modpack-manifest-format.md";
  slug: "fr/modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/offline-mode.md": {
	id: "fr/offline-mode.md";
  slug: "fr/offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/runtime-path-vs-app-data.md": {
	id: "fr/runtime-path-vs-app-data.md";
  slug: "fr/runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/save-format-compatibility.md": {
	id: "fr/save-format-compatibility.md";
  slug: "fr/save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"fr/troubleshooting.md": {
	id: "fr/troubleshooting.md";
  slug: "fr/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"gdl-account-vs-microsoft-account.md": {
	id: "gdl-account-vs-microsoft-account.md";
  slug: "gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"gdlauncher-vs-gdlauncher-carbon.md": {
	id: "gdlauncher-vs-gdlauncher-carbon.md";
  slug: "gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"installation.md": {
	id: "installation.md";
  slug: "installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"instance-folder-anatomy.md": {
	id: "instance-folder-anatomy.md";
  slug: "instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/authentication-errors.md": {
	id: "it/authentication-errors.md";
  slug: "it/authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/gdl-account-vs-microsoft-account.md": {
	id: "it/gdl-account-vs-microsoft-account.md";
  slug: "it/gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/gdlauncher-vs-gdlauncher-carbon.md": {
	id: "it/gdlauncher-vs-gdlauncher-carbon.md";
  slug: "it/gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/installation.md": {
	id: "it/installation.md";
  slug: "it/installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/instance-folder-anatomy.md": {
	id: "it/instance-folder-anatomy.md";
  slug: "it/instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/java-memory-and-gc.md": {
	id: "it/java-memory-and-gc.md";
  slug: "it/java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/locked-modpack-instances.md": {
	id: "it/locked-modpack-instances.md";
  slug: "it/locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/migration-from-legacy-gdlauncher.md": {
	id: "it/migration-from-legacy-gdlauncher.md";
  slug: "it/migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/mod-loaders-compared.md": {
	id: "it/mod-loaders-compared.md";
  slug: "it/mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/modpack-manifest-format.md": {
	id: "it/modpack-manifest-format.md";
  slug: "it/modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/offline-mode.md": {
	id: "it/offline-mode.md";
  slug: "it/offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/runtime-path-vs-app-data.md": {
	id: "it/runtime-path-vs-app-data.md";
  slug: "it/runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/save-format-compatibility.md": {
	id: "it/save-format-compatibility.md";
  slug: "it/save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"it/troubleshooting.md": {
	id: "it/troubleshooting.md";
  slug: "it/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/authentication-errors.md": {
	id: "ja/authentication-errors.md";
  slug: "ja/authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/gdl-account-vs-microsoft-account.md": {
	id: "ja/gdl-account-vs-microsoft-account.md";
  slug: "ja/gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/gdlauncher-vs-gdlauncher-carbon.md": {
	id: "ja/gdlauncher-vs-gdlauncher-carbon.md";
  slug: "ja/gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/installation.md": {
	id: "ja/installation.md";
  slug: "ja/installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/instance-folder-anatomy.md": {
	id: "ja/instance-folder-anatomy.md";
  slug: "ja/instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/java-memory-and-gc.md": {
	id: "ja/java-memory-and-gc.md";
  slug: "ja/java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/locked-modpack-instances.md": {
	id: "ja/locked-modpack-instances.md";
  slug: "ja/locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/migration-from-legacy-gdlauncher.md": {
	id: "ja/migration-from-legacy-gdlauncher.md";
  slug: "ja/migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/mod-loaders-compared.md": {
	id: "ja/mod-loaders-compared.md";
  slug: "ja/mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/modpack-manifest-format.md": {
	id: "ja/modpack-manifest-format.md";
  slug: "ja/modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/offline-mode.md": {
	id: "ja/offline-mode.md";
  slug: "ja/offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/runtime-path-vs-app-data.md": {
	id: "ja/runtime-path-vs-app-data.md";
  slug: "ja/runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/save-format-compatibility.md": {
	id: "ja/save-format-compatibility.md";
  slug: "ja/save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ja/troubleshooting.md": {
	id: "ja/troubleshooting.md";
  slug: "ja/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"java-memory-and-gc.md": {
	id: "java-memory-and-gc.md";
  slug: "java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/authentication-errors.md": {
	id: "ko/authentication-errors.md";
  slug: "ko/authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/gdl-account-vs-microsoft-account.md": {
	id: "ko/gdl-account-vs-microsoft-account.md";
  slug: "ko/gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/gdlauncher-vs-gdlauncher-carbon.md": {
	id: "ko/gdlauncher-vs-gdlauncher-carbon.md";
  slug: "ko/gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/installation.md": {
	id: "ko/installation.md";
  slug: "ko/installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/instance-folder-anatomy.md": {
	id: "ko/instance-folder-anatomy.md";
  slug: "ko/instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/java-memory-and-gc.md": {
	id: "ko/java-memory-and-gc.md";
  slug: "ko/java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/locked-modpack-instances.md": {
	id: "ko/locked-modpack-instances.md";
  slug: "ko/locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/migration-from-legacy-gdlauncher.md": {
	id: "ko/migration-from-legacy-gdlauncher.md";
  slug: "ko/migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/mod-loaders-compared.md": {
	id: "ko/mod-loaders-compared.md";
  slug: "ko/mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/modpack-manifest-format.md": {
	id: "ko/modpack-manifest-format.md";
  slug: "ko/modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/offline-mode.md": {
	id: "ko/offline-mode.md";
  slug: "ko/offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/runtime-path-vs-app-data.md": {
	id: "ko/runtime-path-vs-app-data.md";
  slug: "ko/runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/save-format-compatibility.md": {
	id: "ko/save-format-compatibility.md";
  slug: "ko/save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"ko/troubleshooting.md": {
	id: "ko/troubleshooting.md";
  slug: "ko/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"locked-modpack-instances.md": {
	id: "locked-modpack-instances.md";
  slug: "locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"migration-from-legacy-gdlauncher.md": {
	id: "migration-from-legacy-gdlauncher.md";
  slug: "migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"mod-loaders-compared.md": {
	id: "mod-loaders-compared.md";
  slug: "mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"modpack-manifest-format.md": {
	id: "modpack-manifest-format.md";
  slug: "modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"offline-mode.md": {
	id: "offline-mode.md";
  slug: "offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/authentication-errors.md": {
	id: "pt-BR/authentication-errors.md";
  slug: "pt-br/authentication-errors";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/gdl-account-vs-microsoft-account.md": {
	id: "pt-BR/gdl-account-vs-microsoft-account.md";
  slug: "pt-br/gdl-account-vs-microsoft-account";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/gdlauncher-vs-gdlauncher-carbon.md": {
	id: "pt-BR/gdlauncher-vs-gdlauncher-carbon.md";
  slug: "pt-br/gdlauncher-vs-gdlauncher-carbon";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/installation.md": {
	id: "pt-BR/installation.md";
  slug: "pt-br/installation";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/instance-folder-anatomy.md": {
	id: "pt-BR/instance-folder-anatomy.md";
  slug: "pt-br/instance-folder-anatomy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/java-memory-and-gc.md": {
	id: "pt-BR/java-memory-and-gc.md";
  slug: "pt-br/java-memory-and-gc";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/locked-modpack-instances.md": {
	id: "pt-BR/locked-modpack-instances.md";
  slug: "pt-br/locked-modpack-instances";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/migration-from-legacy-gdlauncher.md": {
	id: "pt-BR/migration-from-legacy-gdlauncher.md";
  slug: "pt-br/migration-from-legacy-gdlauncher";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/mod-loaders-compared.md": {
	id: "pt-BR/mod-loaders-compared.md";
  slug: "pt-br/mod-loaders-compared";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/modpack-manifest-format.md": {
	id: "pt-BR/modpack-manifest-format.md";
  slug: "pt-br/modpack-manifest-format";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/offline-mode.md": {
	id: "pt-BR/offline-mode.md";
  slug: "pt-br/offline-mode";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/runtime-path-vs-app-data.md": {
	id: "pt-BR/runtime-path-vs-app-data.md";
  slug: "pt-br/runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/save-format-compatibility.md": {
	id: "pt-BR/save-format-compatibility.md";
  slug: "pt-br/save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"pt-BR/troubleshooting.md": {
	id: "pt-BR/troubleshooting.md";
  slug: "pt-br/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"runtime-path-vs-app-data.md": {
	id: "runtime-path-vs-app-data.md";
  slug: "runtime-path-vs-app-data";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"save-format-compatibility.md": {
	id: "save-format-compatibility.md";
  slug: "save-format-compatibility";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"troubleshooting.md": {
	id: "troubleshooting.md";
  slug: "troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
};

	};

	type DataEntryMap = {
		
	};

	type AnyEntryMap = ContentEntryMap & DataEntryMap;

	export type ContentConfig = typeof import("../src/content/config.js");
}
