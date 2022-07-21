# A bit of history

GDLauncher was created back in 2014 by Davide as a way to learn programming and do something fun. Over the years the project grew and the users from just him and a couple of friends became hundreds and then thousands.

Since the main goal of the project was to just get better at programming, it has been rewritten multiple times in multiple languages, and we believe we finally reached our goal.

We are now decently happy with our tech stach and we believe we can deliver meaningful features in the shortest time, while also providing an awesome user experience.
Below you can see GDLauncher's evolution over the years.


<img src="https://cdn.gdlauncher.com/assets/articles/curseforge-partnership-announcement/launcher_evolution.webp" height="400px" alt="Launcher Evolution">

# GDLauncher x Curseforge
We are so thrilled to announce that we just partnered with Curseforge!
Curseforge is going through a very deep internal change after being acquired by Overwolf and we believe that this partnership will benefit all parties, especially the community.

After the decision of allowing mods to opt out from third parties due to them not contributing to the content creator's payments, the Minecraft community really took a hit.

Playing modded minecraft became very stressful and cumbersome, especially for linux users and anyone that didn't like the Curseforge app, and it just shouldn't be. We believe we can make it more fun while also delivering very interesting features.

We also applied and got accepted into Overwolf's funding program, which will allow us to take the next big step and create something amazing!

# How will this partnership impact the launcher? What about the rewrite (Carbon)?
GDLauncher has always been loved for our attention to details and the awesome user experience it provides. Truth is that we can still do so much better. We believe that our product is average at best, very buggy, heavy and is still lacking a lot of important features that a custom launcher like ours, should have. This is where our partnership comes in.

Maintaining open source software has always been challenging, most people (like us) do it just because they love programming and they love creating amazing software. In most cases people have a job, a family, other interests... and there are only 24 hours in a day, so some kind of tradeoff needs to happen and open source is usually what is left out.

Moreover, even though open source software can technically be community driven, in most cases it's not, and GDLauncher is a clear example.

 The vast majority of it has been developed by just 2 of us over the years, taking thousands of hours of work, below you can find an insights of the added/removed lines of code of all contributors.

[TODO: IMAGE OF INSIGHTS]

Wouldn't it be wonderful if we could do it full time, while also providing awesome content to the community? That's the goal of the partnership.

We are dedicating all of our resources towards the development of a brand new version of the launcher called GDLauncher Carbon, which will have full access to CF's APIs while also improving the current experience in every way.

## Will GDLauncher be part of the Overwolf network?
No, GDLauncher will remain a standalone installable/portable executable that does not depend on the overwolf app.

## Will GDLauncher become a paid product?

No, there will always be a free and open source version of GDLauncher. Optional monthly subscriptions might eventually be added later to get access to non-essential proprietary features, but no decision on this has been taken yet.

## Will GDLauncher be owned by CurseForge / Overwolf?

No, GDLauncher will remain completely independent in its decisions and development. Overwolf and CurseForge will not be involved in any decisions regarding the development of GDLauncher aside from the requirements of the funding.

## I heard GDLauncher Carbon will still use electron, is that true?

Yes, GDLauncher will still use electron. We know there are other options out there, but as of today, we don't believe there is any real alternative to electron that is production-ready.

## Will it be faster? Lighter?

Even though we will still use electron, there are some major differences in the tech stach we're using.
Right now the entire app is written in React, including all the native operations to the operating system. This is an issue since nodeJS is single threaded and running both the UI and business logic on it is not ideal. React is also not very performant when it comes to rendering.
In the rewrite all the native operations will be done in rust in a separate thread, those will all take advantage from rust blazingly fast speed.
The UI will be written in SolidJS which is a super lightweight library with near-native js performance.

## What about translations? Will it support x language?

Translations will have first class support in GDLauncher Carbon, both for the website and the app! We know a lot of our users are not english native speakers and we can't wait to give them the option to use GDLauncher in their favorite language.
The first languages we want to support are English, German, Italian, Japanese and French.
In general there is no reason why we shouldn't support other languages, so if you want to have your language added, let us know in our [discord](https://discord.gdlauncher.com).

## Will it be open source?

The short answer is yes. The more complete answer is below.
We strongly believe in the power of open source to improve the world around us, but choices need to be sustainable long term.

### Code License

We want to keep the project as open as possible while also preventing unfair competition and exploitation of our work.
Our main repo will stay GPLv3 and will have most of our codebase, it will be fully functioning and have all necessary features that you can expect from a simple launcher.
All our proprietary features will be closed source and only available on our official builds.

### Assets License

Assets (images, illustrations, videos, animations...) are all licensed under ARR (All rights reserved).

### Translations License

Translations will be licensed under Creative Commons Zero v1.0 Universal.

## What advantages will it have over the current version?

As already explained in the [history]() section, GDLauncher was originally written with very little knowledge of coding and over time accumulated a non negligible amount of technical debt, to the point that today it's very hard to maintain and add new features to it.

 We can see the current version as a proof of concept of what we could achieve with no resources and little coding knowledge. Now a few years have passed and we are way more experienced and we now also have enough resources to really bring it to the next level.

## How can I contribute?

Yes you heard that right! If you want to join the team you can either

- Contribute as an open source contributor in your spare time.
- Apply to our hiring selection at [here](https://hiring.gdlauncher.com) to work with us full time! We are hiring designers as well as rust and javascript engineers.

## I have another question!
If you got any other questions, please come talk with us on our [Discord](https://discord.gdlauncher.com), we'll be more than happy to answer!