import useSWR from "swr";
import { getContractAddr } from "../lib/beakerState";
import { getAddress, getClient, getSigningClient } from "../lib/client";

export const propose = async (
  title: string,
  description: string,
  msgs: {
    wasm: {
      execute: {
        contract_addr: string;
        msg: string;
        funds: never[];
      };
    };
  }[]
) => {
  const client = await getSigningClient();
  return client.execute(
    await getAddress(),
    getContractAddr("cw3-flex-multisig"),
    {
      propose: {
        title,
        description,
        msgs,
      },
    },
    "auto"
  );
};

export const getProposal = async (proposal_id: number) => {
  const client = await getClient();
  const res = await client.queryContractSmart(
    getContractAddr("cw3-flex-multisig"),
    {
      proposal: {
        proposal_id,
      },
    }
  );

  return res;
};

export const useProposal = (proposal_id: number) =>
  useSWR("/cw3-flex-multisig/proposal", () => getProposal(proposal_id));
