import { Tab, TabList, TabPanel, Tabs } from "@gd/ui"
import { ModalProps } from "../.."
import ModalLayout from "../../ModalLayout"
import { Trans, useTransContext } from "@gd/i18n"
import Custom from "./Custom"
import Import from "./Import"
import { Match, Switch } from "solid-js"

interface Props {
  id?: number
  import?: boolean
}

const InstanceCreation = (props: ModalProps) => {
  const data: () => Props = () => props.data
  const [t] = useTransContext()

  const title = () =>
    data()?.id !== undefined && data()?.id !== null
      ? t("modals:_trn_title.modify_instance")
      : t("modals:_trn_title.new_instance")

  return (
    <ModalLayout noHeader={props.noHeader} title={title()} noPadding={true}>
      <div class="w-140 flex h-full flex-col justify-between">
        <Switch>
          <Match when={data()?.id !== undefined && data()?.id !== null}>
            <Custom data={data()} />
          </Match>
          <Match when={data()?.id === undefined || data()?.id === null}>
            <Tabs defaultIndex={data()?.import ? 1 : 0}>
              <TabList heightClass="h-14">
                <Tab class="w-1/2" centerContent>
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:add-01" />
                    <Trans key="instances:_trn_instance_creation_custom_tab" />
                  </div>
                </Tab>
                <Tab class="w-1/2" centerContent>
                  <div class="flex items-center gap-2">
                    <div class="i-hugeicons:upload-01" />
                    <Trans key="instances:_trn_instance_import_tab" />
                  </div>
                </Tab>
              </TabList>
              <TabPanel>
                <Custom data={data()} />
              </TabPanel>
              <TabPanel>
                <Import />
              </TabPanel>
            </Tabs>
          </Match>
        </Switch>
      </div>
    </ModalLayout>
  )
}

export default InstanceCreation
