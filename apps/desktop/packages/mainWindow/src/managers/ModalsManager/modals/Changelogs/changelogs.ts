export type ChangelogEntry = {
  title: string;
  description: string;
};

export type Changelog = {
  new: ChangelogEntry[];
  fixed: ChangelogEntry[];
  improved: ChangelogEntry[];
};

const changelogs: Changelog = {
  new: [],
  fixed: [],
  improved: []
};

export default changelogs;
