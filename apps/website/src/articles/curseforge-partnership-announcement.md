# A Bit of History

GDLauncher was created back in 2014 by Davide as a way to learn programming and do something fun. Over the years the project has grown, and the users have gone from just Davide and a couple of friends, to hundreds, and then thousands today.

Since the main goal of the project was to just get better at programming, it has been rewritten multiple times in multiple languages and frameworks, but we believe we have finally reached our goal.

We are now decently happy with our tech stack, we believe we will soon be able deliver meaningful features in a short amount of time while also providing an awesome user experience.

Below, you can see GDLauncher's design evolution over the years.

<img src="https://cdn.gdlauncher.com/assets/articles/curseforge-partnership-announcement/launcher_evolution.webp" alt="GDLauncher's Evolution">

# GDLauncher x CurseForge

We are so thrilled to announce that we have just partnered up with Curseforge!
Curseforge is going through a very deep internal change after being acquired by Overwolf, and we believe that this partnership will benefit all parties, especially the community.

After the decision of allowing mods to opt out from third parties due to them not contributing to the content creators' payments, the Minecraft community really took a hit.

Playing modded Minecraft became very stressful and cumbersome, especially for Linux users and anyone that didn't like the Curseforge app, it just shouldn't be like this. We believe we can make modded Minecraft more convenient and fun, while also delivering very interesting and useful features that don't exist anywhere else.

We also applied and got accepted into Overwolf's funding program, which will allow us to take the next big step and create a truly amazing product!

<img src="https://cdn.gdlauncher.com/assets/articles/curseforge-partnership-announcement/GDl_Dev_meme.webp" alt="GDLauncher's Slow Development">

One of the more frequent complaint we received is the lack of consistency in our releases and bug fixes, but GDLauncher has always been loved for our attention to detail and the awesome user experience it provides, but the truth is that we can still do so much better. We believe that our product is average at best, rather buggy, heavy, and still lacking in important features that a custom launcher should have. This is where our partnership comes in.

It will give us the resources to bring the development of GDLauncher to the next level.

Wouldn't it be wonderful if we could work on it full time, while also providing awesome content to the community? That's the goal of this partnership.

We are dedicating all of our resources towards the development of a brand new version of the launcher called GDLauncher Carbon, which will have full access to CurseForge's APIs, including third party opted out mods, while also improving the current experience in every way.

# FAQ

## How will this partnership impact the current launcher? What about the rewrite, Carbon?
The current launcher will not be too impacted from this partnership aside from the fact that with our latest release (v1.2.0) coming today will have full access to curseforge's APIs.

All the resources coming from our partnership will go towards the development of GDLauncher Carbon. We don't have an ETA yet and we will publish updates in the next few weeks on our [Discord](https://discord.gdlauncher.com).
After GDLauncher Carbon will be ready to go in production, the current release of  GDLauncher will be archieved. We will probably change the license to MIT so anyone will be free to do (mostly) whatever they want with it!

## Will GDLauncher be part of the Overwolf network?

No, GDLauncher will remain a standalone installable/portable executable that does not depend on any overwolf apps.

## Will GDLauncher become a paid product?

No, there will always be a free version of GDLauncher.
However in order to support us, the developers of GDLauncher, and mod(pack) developers on CurseForge, there will be non-intrusive advertisements integrated within the launcher.
Optional monthly subscriptions might be added later to access non-essential features and hide ads, but no decision on this has been taken yet.

## Will GDLauncher be owned by CurseForge / Overwolf?

No, GDLauncher will remain completely independent in its decisions and development.
Although we will become a 2nd-party, Overwolf and CurseForge will not be involved in any decisions regarding the development of GDLauncher aside from the requirements of the funding.

## I heard that Carbon will still use Electron, is that true?

Yes, GDLauncher Carbon will still use Electron. We know there are other options out there, but as of today we don't believe there is any real alternative to Electron that is production-ready.
However because of our Rust backend and web-based frontend, we will be able to switch between web-based application frameworks (e.g. Tauri) without too much fuss if we see the opportunity in the future.

## Will it be faster, lighter, look better?

Even though we will still use Electron, there are some major differences in the tech stack we will be using.

Right now the entire app is written in (unoptimized) React, including all the system operations, networking, logic, etc. This is an issue since NodeJS is single threaded and running both the UI and business logic on it is not ideal. React is also not very performant when it comes to rendering.

In the rewrite, all the native operations will be done in [Rust](https://rust-lang.org) on a separate thread, taking advantage of Rust's blazingly fast speed. The UI will be written in [SolidJS](https://solidjs.com) which is a super lightweight library with close to vanills-JS performance.

## What about translations? Will it support \_\_ language?

Translations will have first class support in GDLauncher Carbon, both for the website and the app! We know a lot of our users are not english native speakers and we can't wait to give them the option to use GDLauncher in their favorite language.

The first languages we want to support are English, German, Italian, Japanese and French.
There is no reason why we shouldn't support other languages, so if you would like to have your language added let us know on our [discord](https://discord.gdlauncher.com). We will also look into crowd-sourcing translations in some way in the future.

## Will GDLauncher still be open source?

We strongly believe in the power of open source to improve the world around us, but choices need to be made to be sustainable in the long term.
The short answer is yes, but there are more details explained below.

### Our goal

Our goal is to keep the source code as open as possible, so that everyone can inspect, study and possibly even contribute to the code, while also trying to make the project sustainable long term.

### Open Source is awesome

Open source is awesome, it allows people to learn and study from someone elses code, fork projects that are no longer maintained and even use them into their own projects! It also forces continuous innovation and inspires people to write cleaner and better code. As we said, we strongly believe in the power of open source and a lot of times making your project open source is the correct answer!

### Open Source is not always awesome

Despite its strengths, there are some dark sides to open source. Many projects are riddled with bugs, poorly documented, and sometimes even dangerous to use. The web is littered with abandoned projects that once seemed promising. It's like a clearance bin you'd find at a discount store; there may be some treasures in there, but you'll have to dig through a lot of undesirables to find the gems. In an industry that's inundated weekly with "hot new" libraries and projects, all just a click away for free, it can be tough to figure out which ones to gamble on.

### Commitment to the project

Some projects are one-trick ponies that don't require ongoing commitment. For example, a math library that performs matrix operations or a formatting library. If the author abandons the project, it's no big deal. Other projects like ours require continuous work to keep up to date with all the external dependencies we have like CurseForge, all the changes happening in Minecraft itself, all the external modloaders and so on.

### Hampered by success

Paradoxically, success is the very thing that kills many open source projects because they don't have a funding mechanism to underwrite all the demands. The project that was once the twinkle in the author's eyes often ends up being a thorn in their side. They can't afford (or don't really want) to keep up with the demands. That's not to say that all open source projects suffer this fate. We have the utmost respect for open source authors, and we don't mean to diminish anyone's hard work or generosity.

### Sustainable

Maintaining open source software has always been challenging, most people (like us) do it just because they love programming and love creating amazing software. However, most people have a job, family, other interests... and there is only so much time we can spare, so some kind of tradeoff needs to happen and open source is usually what is left out.

Moreover, even though open source software can technically be community driven, in most cases it's not, and GDLauncher is a clear example. The vast majority of it has been developed by just 2 of us over the years, taking thousands of hours of work.
Below you can find insights of the added/removed lines of code of all contributors.

<img src="https://cdn.gdlauncher.com/assets/articles/curseforge-partnership-announcement/gdl_contributions.webp" alt="GDLauncher's contributions">

### More about this

If you want to dig deeper into why we chose this direction, please read [this]() article. We also copied parts of that article because we believe it explains it very well.

### Why not open core?

Open core was one of our options, but we discarded it because of the development complexity it brings and the fact that we couldn't make the entire codebase available, which was a no-no.

### Code License

We want to keep the project as open as possible while also preventing unfair competition and exploitation of our work.
Our main repo will stay licensed under `BSL 1.1`. [EXPLAIN MORE ABOUT IT]

### Asset License

Assets (branding, images, illustrations, videos, animations, etc) are all licensed under All Rights Reserved (`ARR`).

### Translations' License

Translations will be licensed under Creative Commons Zero (`CC0`) v1.0 Universal.

## What advantages will the rewrite have over the current version?

As already explained in the [history](#a-bit-of-history) section, GDLauncher was originally written with little knowledge of coding, and has accumulated a non-negligible amount of technical debt, to the point where today it is very hard to maintain and add new features to it.

You can look at the current version as a proof of concept of what we can achieve with no resources and little coding knowledge. A few years have now passed, we are way more experienced, and we now also have enough resources to really bring it to the next level.

## How can I contribute?

Yes you heard that right! If you want to join the team, you can either

- Contribute as an open source contributor in your spare time
- Apply to our hiring selection at [here](https://hiring.gdlauncher.com) to work with us full time! We are hiring designers, and Rust and JavaScript engineers

## I have more questions!

If you have got any other questions, you are welcome to talk with us on our [Discord](https://discord.gdlauncher.com), we are more than happy to clarify your doubts and queries!
