import { ModalProps, useModal } from ".."
import ModalLayout from "../ModalLayout"
import { Trans } from "@gd/i18n"
import { Button } from "@gd/ui"
import { PlaceholderGorilla } from "@/components/PlaceholderGorilla"

const WhyAreAdsNeeded = (props: ModalProps) => {
  const modalsContext = useModal()

  return (
    <ModalLayout
      noHeader={props.noHeader}
      title={props.title}
      height="h-[36rem]"
      width="w-[52rem]"
    >
      <div class="h-full overflow-y-auto">
        <div class="space-y-6 px-8 py-6">
          {/* Gorilla Mascot */}
          <div class="flex justify-center">
            <PlaceholderGorilla
              size={8}
              variant="Thoughtful Gorilla - Explaining Support"
            />
          </div>

          {/* Hero Introduction */}
          <div class="bg-darkSlate-700/50 border-darkSlate-600 flex items-start gap-6 rounded-lg border p-6">
            <div class="bg-darkSlate-800 border-primary-500/30 flex h-16 w-16 shrink-0 items-center justify-center rounded-xl border-2 shadow-[0_0_20px_rgba(139,92,246,0.2)]">
              <div class="i-hugeicons:customer-support text-primary-500 text-[2.5rem]" />
            </div>
            <div class="flex-1">
              <p class="text-lightSlate-300 text-base leading-relaxed">
                <Trans key="ads:_trn_paragraph-1" />
              </p>
            </div>
          </div>

          {/* Why Development Matters Section */}
          <div class="bg-darkSlate-700 border-darkSlate-600 rounded-lg border p-6">
            <h3 class="text-lightSlate-300 mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider">
              <div class="i-hugeicons:code text-primary-500 text-base" />
              <Trans key="ads:_trn_paragraph-2" />
            </h3>
            <div class="grid grid-cols-3 gap-6">
              {/* Card 1 - Minecraft Updates */}
              <div class="bg-darkSlate-800 border-darkSlate-600 group flex flex-col items-center gap-3 rounded-lg border p-4 transition-all duration-300 hover:border-emerald-500/30">
                <div class="bg-darkSlate-700 flex h-14 w-14 items-center justify-center rounded-xl border border-emerald-500/30 transition-transform duration-200 group-hover:scale-110">
                  <div class="i-hugeicons:rocket-01 text-emerald-500 text-[2rem]" />
                </div>
                <p class="text-lightSlate-400 text-center text-sm leading-relaxed">
                  <Trans key="ads:_trn_paragraph-2-list-element-1" />
                </p>
              </div>

              {/* Card 2 - Compatibility */}
              <div class="bg-darkSlate-800 border-darkSlate-600 hover:border-primary-500/30 group flex flex-col items-center gap-3 rounded-lg border p-4 transition-all duration-300">
                <div class="bg-darkSlate-700 border-primary-500/30 flex h-14 w-14 items-center justify-center rounded-xl border transition-transform duration-200 group-hover:scale-110">
                  <div class="i-hugeicons:tick-02 text-primary-500 text-[2rem]" />
                </div>
                <p class="text-lightSlate-400 text-center text-sm leading-relaxed">
                  <Trans key="ads:_trn_paragraph-2-list-element-2" />
                </p>
              </div>

              {/* Card 3 - Enhancement */}
              <div class="bg-darkSlate-800 border-darkSlate-600 group flex flex-col items-center gap-3 rounded-lg border p-4 transition-all duration-300 hover:border-blue-500/30">
                <div class="bg-darkSlate-700 flex h-14 w-14 items-center justify-center rounded-xl border border-blue-500/30 transition-transform duration-200 group-hover:scale-110">
                  <div class="i-hugeicons:sparkles text-blue-500 text-[2rem]" />
                </div>
                <p class="text-lightSlate-400 text-center text-sm leading-relaxed">
                  <Trans key="ads:_trn_paragraph-2-list-element-3" />
                </p>
              </div>
            </div>
          </div>

          {/* What Support Enables Section */}
          <div class="bg-darkSlate-700 border-darkSlate-600 rounded-lg border p-6">
            <h3 class="text-lightSlate-300 mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider">
              <div class="i-hugeicons:dollar-circle text-primary-500 text-base" />
              <Trans key="ads:_trn_paragraph-3" />
            </h3>
            <div class="grid grid-cols-3 gap-6">
              {/* Card 1 - Team Support */}
              <div class="bg-darkSlate-800 border-darkSlate-600 group flex flex-col items-center gap-3 rounded-lg border p-4 transition-all duration-300 hover:border-purple-500/30">
                <div class="bg-darkSlate-700 flex h-14 w-14 items-center justify-center rounded-xl border border-purple-500/30 transition-transform duration-200 group-hover:scale-110">
                  <div class="i-hugeicons:user-multiple-02 text-purple-500 text-[2rem]" />
                </div>
                <p class="text-lightSlate-400 text-center text-sm leading-relaxed">
                  <Trans key="ads:_trn_paragraph-3-list-element-1" />
                </p>
              </div>

              {/* Card 2 - Research & Innovation */}
              <div class="bg-darkSlate-800 border-darkSlate-600 group flex flex-col items-center gap-3 rounded-lg border p-4 transition-all duration-300 hover:border-cyan-500/30">
                <div class="bg-darkSlate-700 flex h-14 w-14 items-center justify-center rounded-xl border border-cyan-500/30 transition-transform duration-200 group-hover:scale-110">
                  <div class="i-hugeicons:bulb text-cyan-500 text-[2rem]" />
                </div>
                <p class="text-lightSlate-400 text-center text-sm leading-relaxed">
                  <Trans key="ads:_trn_paragraph-3-list-element-2" />
                </p>
              </div>

              {/* Card 3 - Full-time Development */}
              <div class="bg-darkSlate-800 border-darkSlate-600 group flex flex-col items-center gap-3 rounded-lg border p-4 transition-all duration-300 hover:border-pink-500/30">
                <div class="bg-darkSlate-700 flex h-14 w-14 items-center justify-center rounded-xl border border-pink-500/30 transition-transform duration-200 group-hover:scale-110">
                  <div class="i-hugeicons:clock-01 text-pink-500 text-[2rem]" />
                </div>
                <p class="text-lightSlate-400 text-center text-sm leading-relaxed">
                  <Trans key="ads:_trn_paragraph-3-list-element-3" />
                </p>
              </div>
            </div>
          </div>

          {/* Revenue Transparency Section */}
          <div class="bg-darkSlate-700 border-darkSlate-600 rounded-lg border p-6">
            <h3 class="text-lightSlate-300 mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider">
              <div class="i-hugeicons:pie-chart text-primary-500 text-base" />
              <Trans key="ads:_trn_revenue_transparency_title" />
            </h3>
            <div class="space-y-4">
              {/* Visual Revenue Bar */}
              <div class="bg-darkSlate-800 border-darkSlate-600 relative h-12 overflow-hidden rounded-lg border">
                <div class="absolute inset-0 flex">
                  <div class="from-primary-500 to-primary-600 flex w-[35%] items-center justify-center bg-gradient-to-r text-sm font-semibold text-white">
                    35%
                  </div>
                  <div class="from-darkSlate-600 to-darkSlate-700 text-lightSlate-300 flex flex-1 items-center justify-center bg-gradient-to-r text-sm font-semibold">
                    65%
                  </div>
                </div>
              </div>

              {/* Labels */}
              <div class="grid grid-cols-2 gap-4">
                <div class="flex items-start gap-3">
                  <div class="bg-primary-500/20 flex h-10 w-10 shrink-0 items-center justify-center rounded-lg p-1.5">
                    <img
                      src="/assets/images/gdlauncher_logo.svg"
                      alt="GDLauncher"
                      class="h-full w-full object-contain"
                    />
                  </div>
                  <div class="flex-1">
                    <div class="text-lightSlate-50 mb-1 text-sm font-semibold">
                      <Trans key="ads:_trn_revenue_split_gdl_title" />
                    </div>
                    <div class="text-lightSlate-500 text-xs">
                      <Trans key="ads:_trn_revenue_split_gdl_desc" />
                    </div>
                  </div>
                </div>
                <div class="flex items-start gap-3">
                  <div class="bg-darkSlate-600 flex h-10 w-10 shrink-0 items-center justify-center rounded-lg">
                    <div class="i-hugeicons:user-star-01 text-lightSlate-400 text-xl" />
                  </div>
                  <div class="flex-1">
                    <div class="text-lightSlate-50 mb-1 text-sm font-semibold">
                      <Trans key="ads:_trn_revenue_split_others_title" />
                    </div>
                    <div class="text-lightSlate-500 text-xs">
                      <Trans key="ads:_trn_revenue_split_others_desc" />
                    </div>
                  </div>
                </div>
              </div>

              <p class="text-lightSlate-400 mt-4 text-sm leading-relaxed">
                <Trans key="ads:_trn_paragraph-4" />
              </p>
            </div>
          </div>

          {/* Ad-Free Options Section */}
          <div class="bg-darkSlate-700 border-darkSlate-600 rounded-lg border p-6">
            <div class="grid grid-cols-2 gap-6">
              {/* Subscription Option */}
              <div class="bg-primary-500/5 border-primary-500/20 flex items-start gap-4 rounded-lg border p-4">
                <div class="bg-primary-500/20 flex h-12 w-12 shrink-0 items-center justify-center rounded-lg">
                  <div class="i-hugeicons:award-01 text-primary-500 text-2xl" />
                </div>
                <div class="flex-1">
                  <p class="text-lightSlate-300 text-sm leading-relaxed">
                    <Trans key="ads:_trn_paragraph-5" />
                  </p>
                </div>
              </div>

              {/* Ad Controls */}
              <div class="bg-darkSlate-600/50 border-darkSlate-500 flex items-start gap-4 rounded-lg border p-4">
                <div class="bg-darkSlate-700 flex h-12 w-12 shrink-0 items-center justify-center rounded-lg">
                  <div class="i-hugeicons:eye text-lightSlate-400 text-2xl" />
                </div>
                <div class="flex-1">
                  <p class="text-lightSlate-300 text-sm leading-relaxed">
                    <Trans key="ads:_trn_paragraph-6" />
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Thank You Section */}
          <div class="space-y-4 py-6 text-center">
            <div class="mb-2 flex items-center justify-center gap-2">
              <div class="i-hugeicons:thumbs-up text-primary-500 text-2xl" />
            </div>
            <p class="text-lightSlate-300 mx-auto max-w-2xl text-base leading-relaxed">
              <Trans key="ads:_trn_paragraph-7" />
            </p>
            <p class="text-lightSlate-400 mt-4 text-sm italic">
              <Trans key="ads:_trn_paragraph-8" />
            </p>

            <div class="mt-6">
              <Button
                type="primary"
                onClick={() => modalsContext?.closeModal()}
                class="px-8 py-3 font-semibold transition-all hover:scale-105 active:scale-95"
              >
                <Trans key="ads:_trn_got_it_thanks" />
              </Button>
            </div>
          </div>
        </div>
      </div>
    </ModalLayout>
  )
}

export default WhyAreAdsNeeded
