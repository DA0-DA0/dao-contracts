package cwusdc_test

import (
	stdMath "github.com/cosmwasm/cosmwasm-go/std/math"
)

type InstantiateMsg struct {
	Denom string `json:"denom"`
}

type ExecuteMsg struct {
	ChangeTokenFactoryAdmin *ChangeTokenFactoryAdmin `json:"change_token_factory_admin,omitempty"`
	ChangeContractOwner     *ChangeContractOwner     `json:"change_contract_owner,omitempty"`
	SetMinter               *SetMinter               `json:"set_minter,omitempty"`
	SetBurner               *SetBurner               `json:"set_burner,omitempty"`
	SetBlacklister          *SetBlacklister          `json:"set_blacklister,omitempty"`
	SetFreezer              *SetFreezer              `json:"set_freezer,omitempty"`
	Mint                    *Mint                    `json:"mint,omitempty"`
	Burn                    *Burn                    `json:"burn,omitempty"`
	Blacklist               *Blacklist               `json:"blacklist,omitempty"`
	Freeze                  *Freeze                  `json:"freeze,omitempty"`
}

type ChangeTokenFactoryAdmin struct {
	NewAdmin string `json:"new_admin"`
}

type ChangeContractOwner struct {
	NewOwner string `json:"new_owner"`
}

type SetMinter struct {
	Address   string          `json:"address"`
	Allowance stdMath.Uint128 `json:"allowance"`
}

type SetBurner struct {
	Address   string          `json:"address"`
	Allowance stdMath.Uint128 `json:"allowance"`
}

type SetBlacklister struct {
	Address string `json:"address"`
	Status  bool   `json:"status"`
}

type SetFreezer struct {
	Address string `json:"address"`
	Status  bool   `json:"status"`
}

type Mint struct {
	ToAddress string          `json:"to_address"`
	Amount    stdMath.Uint128 `json:"amount"`
}

type Burn struct {
	Amount stdMath.Uint128 `json:"amount"`
}

type Blacklist struct {
	Address string `json:"address"`
	Status  bool   `json:"status"`
}

type Freeze struct {
	Status bool `json:"status"`
}

type QueryMsg struct {
	IsFrozen              *IsFrozen              `json:"is_frozen,omitempty"`
	Denom                 *Denom                 `json:"denom,omitempty"`
	Owner                 *Owner                 `json:"owner,omitempty"`
	BurnAllowance         *BurnAllowance         `json:"burn_allowance,omitempty"`
	BurnAllowances        *BurnAllowances        `json:"burn_allowances,omitempty"`
	MintAllowance         *MintAllowance         `json:"mint_allowance,omitempty"`
	MintAllowances        *MintAllowances        `json:"mint_allowances,omitempty"`
	IsBlacklisted         *IsBlacklisted         `json:"is_blacklisted,omitempty"`
	GetBlacklist          *GetBlacklist          `json:"blacklist,omitempty"`
	IsBlacklister         *IsBlacklister         `json:"is_blacklister,omitempty"`
	BlacklisterAllowances *BlacklisterAllowances `json:"blacklister_allowances,omitempty"`
	IsFreezer             *IsFreezer             `json:"is_freezer,omitempty"`
	FreezerAllowances     *FreezerAllowances     `json:"freezer_allowances,omitempty"`
}

type IsFrozen struct{}

type Denom struct{}

type Owner struct{}

type BurnAllowance struct {
	Address string `json:"address"`
}

type BurnAllowances struct {
	StartAfter *string `json:"start_after,omitempty"`
	Limit      *uint32 `json:"limit,omitempty"`
}

type MintAllowance struct {
	Address string `json:"address"`
}

type MintAllowances struct {
	StartAfter *string `json:"start_after,omitempty"`
	Limit      *uint32 `json:"limit,omitempty"`
}

type IsBlacklisted struct {
	Address string `json:"address"`
}

type GetBlacklist struct {
	StartAfter *string `json:"start_after,omitempty"`
	Limit      *uint32 `json:"limit,omitempty"`
}

type IsBlacklister struct {
	Address string `json:"address"`
}

type BlacklisterAllowances struct {
	StartAfter *string `json:"start_after,omitempty"`
	Limit      *uint32 `json:"limit,omitempty"`
}

type IsFreezer struct {
	Address string `json:"address"`
}

type FreezerAllowances struct {
	StartAfter *string `json:"start_after,omitempty"`
	Limit      *uint32 `json:"limit,omitempty"`
}

// ---

type IsFrozenResponse struct {
	IsFrozen bool `json:"is_frozen"`
}

type DenomResponse struct {
	Denom string `json:"denom"`
}

type OwnerResponse struct {
	Address string `json:"address"`
}

type AllowanceResponse struct {
	Allowance stdMath.Uint128 `json:"allowance"`
}

type AllowanceInfo struct {
	Address   string          `json:"address"`
	Allowance stdMath.Uint128 `json:"allowance"`
}

type StatusResponse struct {
	Status bool `json:"status"`
}

type StatusInfo struct {
	Address string `json:"address"`
	Status  bool   `json:"status"`
}

type BlacklistResponse struct {
	Blacklist []StatusInfo `json:"blacklist"`
}

type BlacklisterAllowancesResponse struct {
	Blacklisters []StatusInfo `json:"blacklisters"`
}

type FreezerAllowancesResponse struct {
	Freezers []StatusInfo `json:"freezers"`
}
