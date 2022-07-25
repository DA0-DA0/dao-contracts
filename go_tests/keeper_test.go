package cwusdc_test

import (
	"fmt"
	"io/ioutil"
	"strings"
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
	wasmFile = "../target/wasm32-unknown-unknown/release/cw_usdc.wasm"
	// wasmFile = "../artifacts/cw_usdc.wasm"
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

func (suite *KeeperTestSuite) InstantiateContract(subdenom string) (contractAddr sdk.AccAddress, fullDenom string) {
	instantateMsg := []byte(fmt.Sprintf("{ \"subdenom\": \"%v\" }", subdenom))

	fmt.Println("testAcc0")
	fmt.Println(suite.TestAccs[0].String())

	contractAddr, _, err := suite.contractKeeper.Instantiate(suite.Ctx, suite.codeId, suite.TestAccs[0], suite.TestAccs[0], instantateMsg, "", sdk.NewCoins(sdk.NewInt64Coin("uosmo", 10000000)))
	suite.Require().NoError(err)

	fullDenom = strings.Join([]string{"factory", contractAddr.String(), subdenom}, "/")

	res, err := suite.queryClient.DenomAuthorityMetadata(suite.Ctx.Context(), &types.QueryDenomAuthorityMetadataRequest{Denom: fullDenom})
	suite.Require().NoError(err)
	suite.Require().Equal(res.AuthorityMetadata.GetAdmin(), contractAddr.String())

	return contractAddr, fullDenom
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
