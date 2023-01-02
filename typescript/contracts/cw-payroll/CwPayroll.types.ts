/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.19.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export interface InstantiateMsg {
  admin?: string | null;
}
export type ExecuteMsg = {
  receive: Cw20ReceiveMsg;
} | {
  create: StreamParams;
} | {
  distribute: {
    id: number;
  };
} | {
  pause: {
    id: number;
  };
} | {
  resume: {
    id: number;
  };
} | {
  cancel: {
    id: number;
  };
} | {
  delegate: {};
} | {
  undelegate: {};
} | {
  redelgate: {};
} | {
  withdraw_rewards: {};
};
export type Uint128 = string;
export type Binary = string;
export type CheckedDenom = {
  native: string;
} | {
  cw20: Addr;
};
export type Addr = string;
export interface Cw20ReceiveMsg {
  amount: Uint128;
  msg: Binary;
  sender: string;
}
export interface StreamParams {
  balance: Uint128;
  denom: CheckedDenom;
  description?: string | null;
  end_time: number;
  recipient: string;
  start_time: number;
  title?: string | null;
}
export type QueryMsg = {
  config: {};
} | {
  get_stream: {
    id: number;
  };
} | {
  list_streams: {
    limit?: number | null;
    start?: number | null;
  };
};
export interface ConfigResponse {
  admin: string;
}
export interface StreamResponse {
  balance: Uint128;
  claimed_balance: Uint128;
  denom: CheckedDenom;
  description?: string | null;
  end_time: number;
  id: number;
  paused: boolean;
  paused_duration?: number | null;
  paused_time?: number | null;
  recipient: string;
  start_time: number;
  title?: string | null;
}
export interface ListStreamsResponse {
  streams: StreamResponse[];
}