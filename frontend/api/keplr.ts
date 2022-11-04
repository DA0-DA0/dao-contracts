import useSWR from "swr";
import { getAddress } from "../lib/client";

export const useAddress = () => useSWR(`address`, () => getAddress());
