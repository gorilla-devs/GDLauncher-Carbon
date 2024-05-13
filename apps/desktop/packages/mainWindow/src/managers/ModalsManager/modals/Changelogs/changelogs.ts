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
      title: "Increased app crash timeout.",
      description:
        "This is only a temporary fix. The crashes are caused by Prisma Database Client and we are working on migrating to another database client, but it will take some time."
    }
  ],
  improved: []
};

export default changelogs;
