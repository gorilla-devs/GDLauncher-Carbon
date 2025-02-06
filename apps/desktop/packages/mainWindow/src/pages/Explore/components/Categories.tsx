import {
  NavigationMenu,
  NavigationMenuContent,
  NavigationMenuDescription,
  NavigationMenuItem,
  NavigationMenuItemLabel,
  NavigationMenuLink,
  NavigationMenuTrigger
} from "@gd/ui"
import { ParentProps } from "solid-js"
import CurseforgeLogo from "/assets/images/icons/curseforge_logo.svg"
import ModrinthLogo from "/assets/images/icons/modrinth_logo.svg"

const ListItem = (props: ParentProps<{ title: string; href: string }>) => {
  return (
    <NavigationMenuLink
      href={"#"}
      class="block select-none space-y-1 rounded-md p-3 leading-none no-underline outline-none transition-[box-shadow,background-color] duration-200 hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent focus-visible:text-accent-foreground focus-visible:outline-none focus-visible:ring-[1.5px] focus-visible:ring-ring"
    >
      <NavigationMenuItemLabel class="text-sm font-medium leading-none">
        {props.title}
      </NavigationMenuItemLabel>
      <NavigationMenuDescription class="line-clamp-2 text-sm leading-snug text-muted-foreground">
        {props.children}
      </NavigationMenuDescription>
    </NavigationMenuLink>
  )
}

export default function Categories() {
  return (
    <div class="flex w-full justify-center gap-8 rounded-xl">
      <NavigationMenu>
        <NavigationMenuItem>
          <NavigationMenuTrigger class="text-4xl transition-[box-shadow,background-color] focus-visible:outline-none focus-visible:ring-1.5px focus-visible:ring-ring data-[expanded]:bg-accent">
            <div class="flex items-center gap-2">
              <img class="w-10" src={CurseforgeLogo} alt="Curseforge Logo" />
              Curseforge
            </div>
          </NavigationMenuTrigger>
          <NavigationMenuContent class="grid w-[400px] gap-3 p-4 md:w-[500px] md:grid-cols-2 lg:w-[600px]">
            <ListItem href={"item.href"} title={"Title"}>
              {"Description"}
            </ListItem>
            <ListItem href={"item.href"} title={"Title"}>
              {"Description"}
            </ListItem>
            <ListItem href={"item.href"} title={"Title"}>
              {"Description"}
            </ListItem>
            <ListItem href={"item.href"} title={"Title"}>
              {"Description"}
            </ListItem>
          </NavigationMenuContent>
        </NavigationMenuItem>
        <NavigationMenuItem>
          <NavigationMenuTrigger class="text-4xl transition-[box-shadow,background-color] focus-visible:outline-none focus-visible:ring-1.5px focus-visible:ring-ring data-[expanded]:bg-accent">
            <div class="flex items-center gap-2">
              <img class="w-10" src={ModrinthLogo} alt="Modrinth Logo" />
              Modrinth
            </div>
          </NavigationMenuTrigger>
          <NavigationMenuContent class="grid w-[400px] gap-3 p-4 md:w-[500px] md:grid-cols-2 lg:w-[600px]">
            <ListItem href={"item.href"} title={"Title"}>
              {"Description 2"}
            </ListItem>
            <ListItem href={"item.href"} title={"Title"}>
              {"Description 3"}
            </ListItem>
            <ListItem href={"item.href"} title={"Title"}>
              {"Description 4"}
            </ListItem>
            <ListItem href={"item.href"} title={"Title"}>
              {"Description 5"}
            </ListItem>
          </NavigationMenuContent>
        </NavigationMenuItem>
      </NavigationMenu>
    </div>
  )
}
