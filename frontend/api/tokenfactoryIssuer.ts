import { contracts } from "cw-tokenfactory-issuer-sdk";
import useSWR from "swr";
import { getContractAddr } from "../lib/beakerState";
import { getAddress, getClient, getSigningClient } from "../lib/client";

export const getTokenIssuerQueryClient = async () => {
  const client = await getClient();
  return new contracts.TokenfactoryIssuer.TokenfactoryIssuerQueryClient(
    client,
    getContractAddr("tokenfactory-issuer")
  );
};

export const getTokenIssuerSigningClient = async () => {
  const client = await getSigningClient();
  const sender = await getAddress();
  return new contracts.TokenfactoryIssuer.TokenfactoryIssuerClient(
    client,
    sender,
    getContractAddr("tokenfactory-issuer")
  );
};

export const useDenom = () =>
  useSWR("/tokenfactory-issuer/denom", async () => {
    const client = await getTokenIssuerQueryClient();
    return await client.denom();
  });

export const useOwner = () =>
  useSWR("/tokenfactory-issuer/owner", async () => {
    const client = await getTokenIssuerQueryClient();
    return await client.owner();
  });

export const useMintAllowances = () =>
  useSWR("/tokenfactory-issuer/mint-allowances", async () => {
    const client = await getTokenIssuerQueryClient();
    return await client.mintAllowances({});
  });
export const useBurnAllowances = () =>
  useSWR("/tokenfactory-issuer/burn-allowances", async () => {
    const client = await getTokenIssuerQueryClient();
    return await client.burnAllowances({});
  });

export const useBlacklisterAllowances = () =>
  useSWR("/tokenfactory-issuer/blackister-allowances", async () => {
    const client = await getTokenIssuerQueryClient();
    return await client.blacklisterAllowances({});
  });

export const useBlacklistees = () =>
  useSWR("/tokenfactory-issuer/blackistees", async () => {
    const client = await getTokenIssuerQueryClient();
    return await client.blacklistees({});
  });
