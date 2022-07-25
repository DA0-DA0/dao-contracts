package cwusdc_test

import (
	stdMath "github.com/cosmwasm/cosmwasm-go/std/math"
)

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
	status  bool   `json:"status"`
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
