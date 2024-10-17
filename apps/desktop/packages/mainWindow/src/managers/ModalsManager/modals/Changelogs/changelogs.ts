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
      title: "GDL accounts are now live.",
      description:
        "A GDL account is just an entry in our database that is linked to a Microsoft account ID (UUID). We use your token to authenticate you, your password is NEVER stored. Having a GDL account will give you access to somw new features."
    },
    {
      title: "Redesigned the whole authentication flow.",
      description:
        "It now features a more user-friendly interface, better error handling, and a better UX overall."
    },
    {
      title: "Reworked theming system.",
      description:
        "It now features 3 hand-picked themes. Custom themes are on the way as well."
    },
    {
      title:
        "As part of the GDL accounts, we also redesigned the accounts management system.",
      description:
        'We added an "Accounts" tab in the settings to easily manage your accounts, as well as your GDL account.'
    },
    {
      title: "Library featured modpack can now be hidden",
      description: "by clicking the eye icon in the top right corner."
    }
  ],
  fixed: [
    {
      title: "Fixed instance names with precomposed UNICODE characters.",
      description:
        "You can now use any character in instance names, including japanese characters, emoji, and any other unicode characters."
    },
    {
      title:
        "Fixed a bug where an instance modloader version would not be updated when changing the modloader."
    },
    {
      title:
        "Fixed microphone not being allowed to be used in instances on MacOS."
    }
  ],
  improved: [
    {
      title: "Instances searchbar is now sticky."
    },
    {
      title: "Updated dependencies & toolchain.",
      description:
        "This basically means more stability and performance, as well as less bugs and security issues."
    },
    {
      title: "Added a small transition when switching between pages."
    },
    {
      title:
        "Internal technical change that should improve performance across pages in some cases."
    },
    {
      title: "Added many micro-transitions",
      description: "to various parts of the app, like the instances page."
    },
    {
      title: "Redesigned news component.",
      description:
        "It now takes up less space, and accomodates for a smaller featured tile. While it's static for now, we're working on a dynamic featured tile."
    },
    {
      title: "Improved network download performance",
      description:
        "We've made some changes to the way we download files, which should improve performance and, more importantly, make them more reliable."
    },
    {
      title: "Improved runtime path migration.",
      description:
        "We've made some changes to the way we migrate the runtime path. The UI now shows the current progress of the operation and will display an error message if the migration fails."
    },
    {
      title: "Potato PC mode now also disables hardware acceleration."
    }
  ]
};

export default changelogs;
