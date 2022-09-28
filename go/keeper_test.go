package tokenfactory_issuer_test

import (
	"fmt"
	"io/ioutil"
	"testing"

	sdk "github.com/cosmos/cosmos-sdk/types"
	"github.com/stretchr/testify/suite"

	wasmkeeper "github.com/CosmWasm/wasmd/x/wasm/keeper"
	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"

	bankkeeper "github.com/cosmos/cosmos-sdk/x/bank/keeper"
	banktypes "github.com/cosmos/cosmos-sdk/x/bank/types"

	"github.com/osmosis-labs/osmosis/v10/app/apptesting"
	"github.com/osmosis-labs/osmosis/v10/x/tokenfactory/keeper"
	"github.com/osmosis-labs/osmosis/v10/x/tokenfactory/types"
)

var (
	// wasmFile = "../target/wasm32-unknown-unknown/release/tokenfactory_issuer.wasm"
	wasmFile = "../artifacts/tokenfactory_issuer.wasm"
)

type KeeperTestSuite struct {
	apptesting.KeeperTestHelper

	queryClient    types.QueryClient
	msgServer      types.MsgServer
	contractKeeper wasmtypes.ContractOpsKeeper
	bankMsgServer  banktypes.MsgServer

	codeId uint64
}

func TestKeeperTestSuite(t *testing.T) {
	suite.Run(t, new(KeeperTestSuite))
}

type SudoAuthorizationPolicy struct{}

func (p SudoAuthorizationPolicy) CanCreateCode(config wasmtypes.AccessConfig, actor sdk.AccAddress) bool {
	return true
}

func (p SudoAuthorizationPolicy) CanInstantiateContract(config wasmtypes.AccessConfig, actor sdk.AccAddress) bool {
	return true
}

func (p SudoAuthorizationPolicy) CanModifyContract(admin, actor sdk.AccAddress) bool {
	return true
}

func (suite *KeeperTestSuite) SetupTest() {
	suite.Setup()

	// Fund every TestAcc with 100 denom creation fees.
	fundAccsAmount := sdk.NewCoins(sdk.NewCoin(types.DefaultParams().DenomCreationFee[0].Denom, types.DefaultParams().DenomCreationFee[0].Amount.MulRaw(100)))
	for _, acc := range suite.TestAccs {
		suite.FundAcc(acc, fundAccsAmount)
	}

	suite.SetupTokenFactory()

	suite.contractKeeper = wasmkeeper.NewPermissionedKeeper(suite.App.WasmKeeper, SudoAuthorizationPolicy{})

	suite.queryClient = types.NewQueryClient(suite.QueryHelper)
	suite.msgServer = keeper.NewMsgServerImpl(*suite.App.TokenFactoryKeeper)

	suite.bankMsgServer = bankkeeper.NewMsgServerImpl(suite.App.BankKeeper)

	// upload wasm code
	wasmCode, err := ioutil.ReadFile(wasmFile)
	suite.Require().NoError(err)
	suite.codeId, err = suite.contractKeeper.Create(suite.Ctx, suite.TestAccs[0], wasmCode, nil)
	suite.Require().NoError(err)
}

func (suite *KeeperTestSuite) CreateTokenAndContract() (denom string, contractAddr sdk.AccAddress) {
	// Create the new denom
	denom, err := suite.App.TokenFactoryKeeper.CreateDenom(suite.Ctx, suite.TestAccs[0].String(), "bitcoin")
	suite.Require().NoError(err)

	// Instantiate a new contract to take over as admin of the new denom
	contractAddr = suite.InstantiateContract(denom)

	// Set the BeforeSendListener of the denom to the new contract
	_, err = suite.msgServer.SetBeforeSendListener(sdk.WrapSDKContext(suite.Ctx), types.NewMsgSetBeforeSendHook(
		suite.TestAccs[0].String(),
		denom, contractAddr.String(),
	))
	suite.Require().NoError(err)

	// Query to make sure the BeforeSendHook was set correctly
	res, err := suite.queryClient.DenomBeforeSendHook(sdk.WrapSDKContext(suite.Ctx), &types.QueryDenomBeforeSendHookRequest{
		Denom: denom,
	})
	suite.Require().NoError(err)
	suite.Require().True(res.CosmwasmAddress == contractAddr.String())

	// Set the admin of token to be the contract
	_, err = suite.msgServer.ChangeAdmin(sdk.WrapSDKContext(suite.Ctx), types.NewMsgChangeAdmin(suite.TestAccs[0].String(), denom, contractAddr.String()))
	suite.Require().NoError(err)

	// Query to make sure the new admin was set correctly
	res2, err := suite.queryClient.DenomAuthorityMetadata(sdk.WrapSDKContext(suite.Ctx), &types.QueryDenomAuthorityMetadataRequest{
		Denom: denom,
	})
	suite.Require().NoError(err)
	suite.Require().True(res2.AuthorityMetadata.Admin == contractAddr.String())

	return denom, contractAddr
}

func (suite *KeeperTestSuite) InstantiateContract(denom string) (contractAddr sdk.AccAddress) {
	instantateMsg := []byte(fmt.Sprintf("{ \"denom\": \"%v\" }", denom))

	contractAddr, _, err := suite.contractKeeper.Instantiate(suite.Ctx, suite.codeId, suite.TestAccs[0], suite.TestAccs[0], instantateMsg, "", sdk.NewCoins())
	suite.Require().NoError(err)

	return contractAddr
}

// func ExcecuteMintMsg(mint) []byte {
// 	return []byte(fmt.Sprintf("{ \"set_freezer\": { \"address\": \"%v\", \"status\": true } }", suite.TestAccs[0].String()))
// }

// ChangeTokenFactoryAdmin { new_admin: String },
//     ChangeContractOwner { new_owner: String },
//     SetMinter { address: String, allowance: Uint128 },
//     SetBurner { address: String, allowance: Uint128 },
//     SetBlacklister { address: String, status: bool },
//     SetFreezer { address: String, status: bool },
//     Mint { to_address: String, amount: Uint128 },
//     Burn { amount: Uint128 },
//     Blacklist { address: String, status: bool },
//     Freeze { status: bool },
