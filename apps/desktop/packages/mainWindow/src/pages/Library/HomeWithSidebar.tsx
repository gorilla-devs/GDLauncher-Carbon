const HomeWithSidebar = () => {
  return (
    <div>
      {/* <div class="overflow-hidden">
        <UnstableCard />
        <Show when={routeData.settings.data?.showNews}>
          <div class="flex gap-4">
            <div class="flex-1 flex-grow">
              <Switch>
                <Match when={(news()?.length || 0) > 0}>
                  <News
                    slides={news()}
                    onClick={(news) => {
                      window.openExternalLink(news.url || "")
                    }}
                    fallBackImg={DefaultImg}
                  />
                </Match>
                <Match when={news.length === 0 || routeData.settings.isLoading}>
                  <Skeleton.news />
                </Match>
              </Switch>
            </div>
            <div class="h-auto w-[1px] bg-darkSlate-400" />
            <FeaturedModpackTile />
          </div>
        </Show>
        <Switch>
          <Match
            when={
              routeData.instances.data &&
              routeData.instances.data.length === 0 &&
              !routeData.instances.isLoading
            }
          >
            <div class="w-full h-full flex flex-col justify-center items-center mt-12">
              <img src={skull} class="w-16 h-16" />
              <p class="text-lightSlate-700 text-center max-w-100">
                <Trans
                  key="instance.no_instances_text"
                  options={{
                    defaultValue:
                      "At the moment there are not instances. Add one to start playing!"
                  }}
                />
              </p>
            </div>
          </Match>
          <Match
            when={
              (routeData.instances.data &&
                routeData.instances.data.length > 0 &&
                !routeData.instances.isLoading) ||
              routeData.instances.isLoading
            }
          >
            <div class="mt-4">
              <Switch>
                <Match
                  when={
                    routeData.instances.data &&
                    routeData.instances.data.length > 0 &&
                    !routeData.instances.isLoading
                  }
                >
                  <Carousel title={t("jump_back_in")}>
                    <For
                      each={routeData.instances.data
                        ?.slice()
                        .sort((a, b) => {
                          return (
                            Date.parse(b.last_played || b.date_created) -
                            Date.parse(a.last_played || a.date_created)
                          )
                        })
                        .slice(0, 5)}
                    >
                      {(instance) => (
                        <Suspense fallback={<Skeleton.instance />}>
                          <InstanceTile
                            size={2}
                            instance={instance}
                            identifier="" // TODO: pass the proper identifier, but we don't have it here
                          />
                        </Suspense>
                      )}
                    </For>
                  </Carousel>
                </Match>
                <Match when={routeData.instances.isLoading}>
                  <Skeleton.instances />
                </Match>
              </Switch>
            </div>
            <div class="my-4">
              <Switch>
                <Match
                  when={
                    routeData.instances.data &&
                    routeData.instances.data.length > 0 &&
                    !routeData.instances.isLoading
                  }
                >
                  <Carousel title={t("have_not_played_in_a_while")}>
                    <For
                      each={routeData.instances.data
                        ?.slice()
                        .sort((a, b) => {
                          return (
                            Date.parse(a.last_played || a.date_created) -
                            Date.parse(b.last_played || b.date_created)
                          )
                        })
                        .slice(0, 5)}
                    >
                      {(instance) => (
                        <Suspense fallback={<Skeleton.instance />}>
                          <InstanceTile
                            size={2}
                            instance={instance}
                            identifier="" // TODO: pass the proper identifier, but we don't have it here
                          />
                        </Suspense>
                      )}
                    </For>
                  </Carousel>
                </Match>
                <Match when={routeData.instances.isLoading}>
                  <Skeleton.instances />
                </Match>
              </Switch>
            </div>
          </Match>
        </Switch>
      </div> */}
    </div>
  )
}

export default HomeWithSidebar
