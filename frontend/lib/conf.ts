export const getRpcEndpoint = () => {
  if (!process.env.NEXT_PUBLIC_HOST) {
    throw Error("`NEXT_PUBLIC_HOST` env variable not found, please set");
  }

  return `${process.env.NEXT_PUBLIC_HOST}/rpc`;
};

export const getChainId = () => {
  if (!process.env.NEXT_PUBLIC_CHAIN_ID) {
    throw Error("`NEXT_PUBLIC_CHAIN_ID` env variable not found, please set");
  }

  return process.env.NEXT_PUBLIC_CHAIN_ID;
};
