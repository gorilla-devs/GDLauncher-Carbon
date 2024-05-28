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
    },
    {
      title: "Fixed import status stalling.",
      description: "The progress should now update correctly."
    },
    {
      title: 'Hid the "unknown" modloader.',
      description:
        "We use it internally to map unknown modloaders but should not be exposed in the UI."
    }
  ],
  improved: [
    {
      title: "The versions are now selectable in the mods browser.",
      description:
        "It will still preselect the correct game version, but you'll be able to override it manually."
    }
  ]
};

export default changelogs;
