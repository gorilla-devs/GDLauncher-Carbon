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
  new: [
    {
      title: "Close app warning",
      description: "when game is running."
    }
  ],
  fixed: [
    {
      title: "Fixed open mods folder",
      description: "not working when mods folder doesn't exist."
    },
    {
      title: "Fixed crash when importing modpack with no thumbnail."
    }
  ],
  improved: [
    {
      title: "Added core module timeout",
      description: "so it doesn't hang forever in some cases."
    },
    {
      title: "Switched from openSSL to rustLS.",
      description: "This should bring more stability across platforms."
    },
    {
      title: "Improved installation errors related to cache."
    },
    {
      title: "Improved resiliency for metadata",
      description:
        "and updated it to all latest versions of minecraft & modloaders."
    },
    {
      title: "Reworked LWJGL system",
      description: "it should now work on more systems."
    },
    {
      title: "Improved how the GDL window is managed in different situations."
    },
    {
      title: "Improved Microsoft token refresh system.",
      description:
        "It should now work more reliably since it refreshes more often."
    }
  ]
};

export default changelogs;
