import { rspc } from "@/utils/rspcClient";

const fetchData = () => {
  let data = rspc.createQuery(() => ["account.enroll.getStatus", null], {
    onError(err) {
      console.log("LOGIN ERROR", err);
    },
  });

  return data;
};

export default fetchData;
