import useSWR from "swr";
import { getOsmoClient } from "../lib/client";

export const balance = async (address: string, denom: string) => {
  const client = await getOsmoClient();
  return client.cosmos.bank.v1beta1.balance({ address, denom });
};

export const denomMetadata = async (denom: string) => {
  const client = await getOsmoClient();
  return client.cosmos.bank.v1beta1.denomMetadata({ denom });
};

export const supply = async (denom: string) => {
  const client = await getOsmoClient();
  return client.cosmos.bank.v1beta1.supplyOf({ denom });
};

export const useBalance = (address: string, denom: string) =>
  useSWR(`/bank/balance/${address}/${denom}`, () => balance(address, denom));

export const useDenomMetadata = (denom: string) =>
  useSWR(`/bank/denom_metadata/${denom}`, () => denomMetadata(denom));

export const useSupply = (denom: string) =>
  useSWR(`/bank/supply/${denom}`, () => supply(denom));
