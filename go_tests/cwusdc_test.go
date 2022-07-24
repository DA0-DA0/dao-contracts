package cwusdc_test

import (
	"fmt"
	"io/ioutil"
	"strings"

	sdk "github.com/cosmos/cosmos-sdk/types"

	banktypes "github.com/cosmos/cosmos-sdk/x/bank/types"
	"github.com/osmosis-labs/osmosis/v7/x/tokenfactory/types"
)

// type SendMsgTestCase struct {
// 	desc       string
// 	msg        func(denom string) *banktypes.MsgSend
// 	expectPass bool
// }

type InstantiateTestCase struct {
	desc       string
	funds      sdk.Coins
	subdenom   string
	expectPass bool
}

func (suite *KeeperTestSuite) TestInstantiateCwUsdcContract() {

	for _, tc := range []struct {
		desc      string
		wasmFile  string
		testCases []InstantiateTestCase
	}{
		{
			desc:     "Contract should only successfully instantiate when the right funds are provided",
			wasmFile: "../artifacts/cw_usdc.wasm",
			testCases: []InstantiateTestCase{

				// currently the tokenfactory charges a cost of 10osmo for creating a new denom, but the contract only checks if you have more than 1000uosmo.
				// this tests show the behaviour
				{
					desc:       "1000 uosmo at initialisation of should not give a contract error, but a tokenfactory error",
					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(1000))},
					subdenom:   "uusdc",
					expectPass: false,
				},
				{
					desc:       "10000000 uosmo at initialisation should be enough for the tokenfactory",
					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(10000000))},
					subdenom:   "uusdc",
					expectPass: true,
				},
				{
					desc:       "No funds at initialisation of contract should fail",
					funds:      []sdk.Coin{},
					subdenom:   "uusdc",
					expectPass: false,
				},
				{
					desc:       "wrong funds at initialisation of contract should fail",
					funds:      []sdk.Coin{sdk.NewCoin("uakt", sdk.NewInt(10000000))},
					subdenom:   "uusdc",
					expectPass: false,
				},
				{
					desc:       "Not enough funds should fail",
					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(100))},
					subdenom:   "uusdc",
					expectPass: false,
				},
			},
		},
	} {
		suite.Run(fmt.Sprintf("Case %s", tc.desc), func() {
			// setup test
			suite.SetupTest()

			// upload and instantiate wasm code
			wasmCode, err := ioutil.ReadFile(tc.wasmFile)
			suite.Require().NoError(err, "test: %v", tc.desc)
			codeID, err := suite.contractKeeper.Create(suite.Ctx, suite.TestAccs[0], wasmCode, nil)
			suite.Require().NoError(err, "test: %v", tc.desc)

			for _, instTc := range tc.testCases {
				instMsg := []byte(fmt.Sprintf("{ \"subdenom\": \"%v\" }", instTc.denom))

				_, _, err := suite.contractKeeper.Instantiate(suite.Ctx, codeID, suite.TestAccs[0], suite.TestAccs[0], instMsg, "", instTc.funds)
				if instTc.expectPass {
					suite.Require().NoError(err, "test: %v", instTc.desc)
				} else {
					suite.Require().Error(err, "test: %v", instTc.desc)
				}
			}
		})
	}
}

type FrozenContractTestCase struct {
	desc       string
	funds      []sdk.Coin
	msg        []byte
	expectPass bool
	denom      string
}

func (suite *KeeperTestSuite) TestFrozenContract() {

	for _, tc := range []struct {
		desc      string
		denom     string
		wasmFile  string
		testCases []FrozenContractTestCase
	}{
		{
			desc:     "Frozen contract should block msgs that use that denom",
			denom:    "frozen",
			wasmFile: "../artifacts/cw_usdc.wasm",
			testCases: []FrozenContractTestCase{

				// currently the tokenfactory charges a cost of 10osmo for creating a new denom, but the contract only checks if you have more than 1000uosmo.
				// this tests show the behaviour
				{
					desc:       "contract should allow transaction of the non frozen token",
					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(1000))},
					denom:      "uosmo",
					expectPass: true,
				},
				{
					desc:       "frozen token should be blocked by contraced",
					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(10000000))},
					denom:      "frozen",
					expectPass: false,
				},
			},
		},
	} {
		suite.Run(fmt.Sprintf("Case %s", tc.desc), func() {
			// setup test
			suite.SetupTest()

			// upload and instantiate wasm code
			wasmCode, err := ioutil.ReadFile(tc.wasmFile)
			suite.Require().NoError(err, "test: %v", tc.desc)
			codeID, err := suite.contractKeeper.Create(suite.Ctx, suite.TestAccs[0], wasmCode, nil)
			suite.Require().NoError(err, "test: %v", tc.desc)
			instMsg := []byte(fmt.Sprintf("{ \"subdenom\": \"%v\" }", tc.denom))
			cosmwasmAddress, _, err := suite.contractKeeper.Instantiate(suite.Ctx, codeID, suite.TestAccs[0], suite.TestAccs[0], instMsg, "", sdk.NewCoins(sdk.NewCoin("uosmo", sdk.NewInt(10_000_000))))
			suite.Require().NoError(err, "test: %v", tc.desc)

			fullDenom := strings.Join([]string{"tokenfactory", cosmwasmAddress.String(), tc.denom}, "/")
			suite.msgServer.Mint(sdk.WrapSDKContext(suite.Ctx), types.NewMsgMint(suite.TestAccs[0].String(), sdk.NewInt64Coin(fullDenom, 1000000000)))

			// set testAcc0 to freezer
			setFreezerMsg := fmt.Sprintf("{ \"set_freezer\": { \"address\": \"%v\", \"status\": true } }", suite.TestAccs[0].String())
			_, err = suite.contractKeeper.Execute(suite.Ctx, cosmwasmAddress, suite.TestAccs[0], []byte(setFreezerMsg), sdk.NewCoins())
			suite.Require().NoError(err, "test %v", tc.desc)

			// set beforesend hook to the new denom
			// TODO: THIS RESULTS IN A Unauthorized account while it should not. The latest version of the contract changes the admin to testAcc[0]
			_, err = suite.msgServer.SetBeforeSendHook(sdk.WrapSDKContext(suite.Ctx), types.NewMsgSetBeforeSendHook(suite.TestAccs[0].String(), fullDenom, cosmwasmAddress.String()))
			suite.Require().NoError(err, "test: %v", tc.desc)

			// freeze contract
			_, err = suite.contractKeeper.Execute(suite.Ctx, cosmwasmAddress, suite.TestAccs[0], []byte("{ \"freeze\": { \"status\": true } }"), sdk.NewCoins())
			suite.Require().NoError(err, "test %v", tc.desc)

			for _, instTc := range tc.testCases {

				_, err = suite.bankMsgServer.Send(sdk.WrapSDKContext(suite.Ctx),
					banktypes.NewMsgSend(
						suite.TestAccs[0],
						suite.TestAccs[1],
						sdk.NewCoins(sdk.NewCoin(fullDenom, sdk.NewInt(1000000))),
					),
				)

				if instTc.expectPass {
					suite.Require().NoError(err, "test: %v", instTc.desc)
				} else {
					suite.Require().Error(err, "test: %v", instTc.desc)
				}
			}
		})
	}
}
