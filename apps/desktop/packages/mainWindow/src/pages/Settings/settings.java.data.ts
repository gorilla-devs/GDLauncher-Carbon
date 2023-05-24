import { rspc } from "@/utils/rspcClient";
import { FESettings } from "@gd/core_module/bindings";
import { CreateQueryResult } from "@tanstack/solid-query";
import { RSPCError } from "@rspc/client";

const SettingsJavaData = ({
  data,
}: {
  data: CreateQueryResult<FESettings, RSPCError>;
}) => {
  let availableJavas = rspc.createQuery(() => ["java.getAvailableJavas"]);
  let javaProfiles = rspc.createQuery(() => ["java.getSystemJavaProfiles"]);
  let totalRam = rspc.createQuery(() => ["systeminfo.getTotalRAM"]);
  return { availableJavas, javaProfiles, settings: data, totalRam };
};

export default SettingsJavaData;
