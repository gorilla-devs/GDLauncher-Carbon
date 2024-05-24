export type ChangelogEntry = {
  title: string;
  description?: string;
};

export type Changelog = {
  new: ChangelogEntry[];
  fixed: ChangelogEntry[];
  improved: ChangelogEntry[];
};

const changelogs: Changelog = {
  new: [],
  fixed: [
    {
      title: "New database migration system.",
      description:
        "Our previous system was hanging for some users. We have now implemented a new system that should be more reliable and faster."
    }
  ],
  improved: []
};

export default changelogs;
