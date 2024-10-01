import { ModalProps } from "../../";
import ModalLayout from "../../ModalLayout";

import { Trans } from "@gd/i18n";
import { For, Match, Show, Switch } from "solid-js";
import changelogs, { Changelog } from "./changelogs";
import { Button } from "@gd/ui";

type SectionProps = {
  type: keyof Changelog;
};

const Section = (props: SectionProps) => {
  const textColor = () => {
    switch (props.type) {
      case "new":
        return "text-green-400";
      case "improved":
        return "text-yellow-400";
      case "fixed":
        return "text-red-400";
    }
  };

  const borderColor = () => {
    switch (props.type) {
      case "new":
        return "border-green-400";
      case "improved":
        return "border-yellow-400";
      case "fixed":
        return "border-red-400";
    }
  };

  const icon = () => {
    switch (props.type) {
      case "new":
        return "i-ri:shining-2-fill";
      case "improved":
        return "i-ri:hammer-fill";
      case "fixed":
        return "i-ri:bug-fill";
    }
  };

  const title = () => {
    switch (props.type) {
      case "new":
        return "NEW";
      case "improved":
        return "IMPROVED";
      case "fixed":
        return "BUG FIXES";
    }
  };

  const list = () => {
    switch (props.type) {
      case "new":
        return changelogs.new;
      case "improved":
        return changelogs.improved;
      case "fixed":
        return changelogs.fixed;
    }
  };

  return (
    <div>
      <div class="flex items-center w-full">
        <div class={`flex-1 border-t-1 ${borderColor()} border-solid`} />
        <span class={`px-3 ${textColor()} flex items-center gap-2 text-xl`}>
          <div class={`inline-block ${icon()} w-4 h-4`} />
          {title()}
        </span>
        <div class={`flex-1 border-t-1 ${borderColor()} border-solid`} />
      </div>
      <div class="py-4">
        <Switch>
          <Match when={list().length === 0}>
            <Trans key="changelogs.no_changes" />
          </Match>
          <Match when={list().length > 0}>
            <ul class="pl-4 my-0">
              <For each={list()}>
                {(item, index) => (
                  <li
                    classList={{
                      "pb-4": index() !== list().length - 1
                    }}
                  >
                    <span class="text-lightSlate-50 font-bold">
                      {item.title}
                    </span>
                    <Show when={item.description}>
                      &nbsp;
                      <span class="text-lightSlate-500">
                        {item.description}
                      </span>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
          </Match>
        </Switch>
      </div>
    </div>
  );
};

const Changelogs = (props: ModalProps) => {
  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props?.title}
      noPadding
      height="h-150"
      width="w-130"
    >
      <div class="w-full h-full overflow-auto px-5 pb-5 box-border">
        <h2>
          <Trans
            key="changelogs.whats_new_in"
            options={{
              version: __APP_VERSION__
            }}
          />
        </h2>
        <Section type="new" />
        <Section type="improved" />
        <Section type="fixed" />
        <div class="rounded-md flex flex-col items-center gap-4">
          <hr class="w-full" />
          <div class="text-brands-twitchWhite">
            <Trans key="changelogs.watch_on_twitch_description" />
          </div>

          <Button
            backgroundColor="bg-brands-twitchPurpleDark"
            onClick={() => {
              window.open("https://twitch.tv/blarfoon", "_blank");
            }}
          >
            <div class="flex items-center justify-center gap-4 animate-wiggle animate-delay-1400 animate-loop">
              <div class="i-ri:twitch-fill w-6 h-6 inline-block" />
              <Trans key="changelogs.watch_on_twitch" />
            </div>
          </Button>
        </div>
      </div>
    </ModalLayout>
  );
};

export default Changelogs;
