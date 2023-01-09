/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.19.0.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

export interface InstantiateMsg {
  owner?: string | null;
  params: UncheckedVestingParams;
}
export type ExecuteMsg = {
  instantiate_payroll_contract: {
    code_id: number;
    instantiate_msg: InstantiateMsg;
    label: string;
  };
};
export type Uint128 = string;
export type UncheckedDenom = {
  native: string;
} | {
  cw20: string;
};
export type Curve = {
  constant: {
    y: Uint128;
    [k: string]: unknown;
  };
} | {
  saturating_linear: SaturatingLinear;
} | {
  piecewise_linear: PiecewiseLinear;
};
export interface UncheckedVestingParams {
  amount: Uint128;
  denom: UncheckedDenom;
  description?: string | null;
  recipient: string;
  title?: string | null;
  vesting_schedule: Curve;
}
export interface SaturatingLinear {
  max_x: number;
  max_y: Uint128;
  min_x: number;
  min_y: Uint128;
  [k: string]: unknown;
}
export interface PiecewiseLinear {
  steps: [number, Uint128][];
  [k: string]: unknown;
}
export type QueryMsg = {
  list_vesting_contracts: {
    limit?: number | null;
    start_after?: string | null;
  };
} | {
  list_vesting_contracts_by_instantiator: {
    instantiator: string;
    limit?: number | null;
    start_after?: string | null;
  };
} | {
  list_vesting_contracts_by_recipient: {
    limit?: number | null;
    recipient: string;
    start_after?: string | null;
  };
};
export interface MigrateMsg {}
export type Addr = string;
export type ArrayOfAddr = Addr[];