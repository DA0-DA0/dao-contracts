import useSWR, { mutate } from "swr";
import { getAddress } from "../lib/client";

export const useAddress = () => useSWR(`address`, () => getAddress());

if (typeof window !== "undefined") {
  window.addEventListener("keplr_keystorechange", async () => {
    console.log("Key store in Keplr is changed. Refetching the account info.");
    mutate("address", await getAddress());
  });
}
