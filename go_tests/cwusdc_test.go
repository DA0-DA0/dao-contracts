package cwusdc_test

import (
	"encoding/json"
	"fmt"

	sdk "github.com/cosmos/cosmos-sdk/types"

	stdMath "github.com/cosmwasm/cosmwasm-go/std/math"

	banktypes "github.com/cosmos/cosmos-sdk/x/bank/types"
)

// type InstantiateTestCase struct {
// 	desc       string
// 	funds      sdk.Coins
// 	subdenom   string
// 	expectPass bool
// }

// func (suite *KeeperTestSuite) TestInstantiateCwUsdcContract() {

// 	for _, tc := range []struct {
// 		desc      string
// 		wasmFile  string
// 		testCases []InstantiateTestCase
// 	}{
// 		{
// 			desc:     "Contract should only successfully instantiate when the right funds are provided",
// 			wasmFile: "../artifacts/cw_usdc.wasm",
// 			testCases: []InstantiateTestCase{

// 				// currently the tokenfactory charges a cost of 10osmo for creating a new denom, but the contract only checks if you have more than 1000uosmo.
// 				// this tests show the behaviour
// 				{
// 					desc:       "1000 uosmo at initialisation of should not give a contract error, but a tokenfactory error",
// 					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(1000))},
// 					subdenom:   "uusdc",
// 					expectPass: false,
// 				},
// 				{
// 					desc:       "10000000 uosmo at initialisation should be enough for the tokenfactory",
// 					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(10000000))},
// 					subdenom:   "uusdc",
// 					expectPass: true,
// 				},
// 				{
// 					desc:       "No funds at initialisation of contract should fail",
// 					funds:      []sdk.Coin{},
// 					subdenom:   "uusdc",
// 					expectPass: false,
// 				},
// 				{
// 					desc:       "wrong funds at initialisation of contract should fail",
// 					funds:      []sdk.Coin{sdk.NewCoin("uakt", sdk.NewInt(10000000))},
// 					subdenom:   "uusdc",
// 					expectPass: false,
// 				},
// 				{
// 					desc:       "Not enough funds should fail",
// 					funds:      []sdk.Coin{sdk.NewCoin("uosmo", sdk.NewInt(100))},
// 					subdenom:   "uusdc",
// 					expectPass: false,
// 				},
// 			},
// 		},
// 	} {
// 		suite.Run(fmt.Sprintf("Case %s", tc.desc), func() {
// 			// setup test
// 			suite.SetupTest()

// 			// upload and instantiate wasm code
// 			wasmCode, err := ioutil.ReadFile(tc.wasmFile)
// 			suite.Require().NoError(err, "test: %v", tc.desc)
// 			codeID, err := suite.contractKeeper.Create(suite.Ctx, suite.TestAccs[0], wasmCode, nil)
// 			suite.Require().NoError(err, "test: %v", tc.desc)

// 			for _, instTc := range tc.testCases {
// 				instMsg := []byte(fmt.Sprintf("{ \"subdenom\": \"%v\" }", instTc.subdenom))

// 				_, _, err := suite.contractKeeper.Instantiate(suite.Ctx, codeID, suite.TestAccs[0], suite.TestAccs[0], instMsg, "", instTc.funds)
// 				if instTc.expectPass {
// 					suite.Require().NoError(err, "test: %v", instTc.desc)
// 				} else {
// 					suite.Require().Error(err, "test: %v", instTc.desc)
// 				}
// 			}
// 		})
// 	}
// }

func (suite *KeeperTestSuite) TestFreeze() {
	for _, tc := range []struct {
		desc       string
		frozen     bool
		bankMsg    func(denom string) *banktypes.MsgSend
		expectPass bool
	}{
		{
			desc:   "if contract is not frozen, should allow transfer of the denom",
			frozen: false,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(sdk.NewInt64Coin(denom, 1)))
			},
			expectPass: true,
		},
		{
			desc:   "if contract is not frozen, should allow transfer of multidenom",
			frozen: false,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin(denom, 1),
					sdk.NewInt64Coin("uosmo", 1),
				))
			},
			expectPass: true,
		},
		{
			desc:   "if contract is frozen, should not transfer of the denom",
			frozen: true,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin(denom, 1),
				))
			},
			expectPass: false,
		},
		{
			desc:   "if contract is frozen, should allow transfer of a different denom",
			frozen: true,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin("uosmo", 1),
				))
			},
			expectPass: true,
		},

		{
			desc:   "if contract is frozen, should not transaction of multidenom",
			frozen: true,
			bankMsg: func(denom string) *banktypes.MsgSend {
				return banktypes.NewMsgSend(suite.TestAccs[0], suite.TestAccs[1], sdk.NewCoins(
					sdk.NewInt64Coin(denom, 1),
					sdk.NewInt64Coin("uosmo", 1),
				))
			},
			expectPass: false,
		},
	} {
		suite.Run(fmt.Sprintf("Case %s", tc.desc), func() {
			// setup test
			suite.SetupTest()
			contractAddr, fullDenom := suite.InstantiateContract("bitcoin")

			// give mint permissions to testAcc0
			setMinterMsg, err := json.Marshal(ExecuteMsg{SetMinter: &SetMinter{Address: suite.TestAccs[0].String(), Allowance: stdMath.MaxUint128()}})
			suite.Require().NoError(err, "test %v", tc.desc)

			fmt.Println(string(setMinterMsg))
			_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], setMinterMsg, sdk.NewCoins())
			suite.Require().NoError(err, "test %v", tc.desc)

			// mint 100000 coins to testAcc0
			mintMsg, err := json.Marshal(ExecuteMsg{Mint: &Mint{ToAddress: suite.TestAccs[0].String(), Amount: stdMath.NewUint128FromUint64(1000000)}})
			suite.Require().NoError(err, "test %v", tc.desc)
			_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], mintMsg, sdk.NewCoins())
			suite.Require().NoError(err, "test %v", err)

			// give testAcc0 freeze permissions
			setFreezerMsg, err := json.Marshal(ExecuteMsg{SetFreezer: &SetFreezer{Address: suite.TestAccs[0].String(), Status: true}})
			suite.Require().NoError(err, "test %v", tc.desc)
			_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], setFreezerMsg, sdk.NewCoins())
			suite.Require().NoError(err, "test %v", tc.desc)

			// if should freeze
			if tc.frozen {
				// freeze contract
				freezeMsg, err := json.Marshal(ExecuteMsg{Freeze: &Freeze{Status: true}})
				_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], freezeMsg, sdk.NewCoins())
				suite.Require().NoError(err, "test %v", tc.desc)
				// TODO: use query to asset is frozen
			} else {
				freezeMsg, err := json.Marshal(ExecuteMsg{Freeze: &Freeze{Status: false}})
				_, err = suite.contractKeeper.Execute(suite.Ctx, contractAddr, suite.TestAccs[0], freezeMsg, sdk.NewCoins())
				suite.Require().NoError(err, "test %v", tc.desc)
			}

			// // set beforesend hook to the new denom
			// // TODO: THIS RESULTS IN A Unauthorized account while it should not. The latest version of the contract changes the admin to testAcc[0]
			// _, err = suite.msgServer.SetBeforeSendHook(sdk.WrapSDKContext(suite.Ctx), types.NewMsgSetBeforeSendHook(suite.TestAccs[0].String(), fullDenom, cosmwasmAddress.String()))
			// suite.Require().NoError(err, "test: %v", tc.desc)

			// // freeze contract
			// _, err = suite.contractKeeper.Execute(suite.Ctx, cosmwasmAddress, suite.TestAccs[0], []byte("{ \"freeze\": { \"status\": true } }"), sdk.NewCoins())
			// suite.Require().NoError(err, "test %v", tc.desc)

			// for _, instTc := range tc.testCases {

			_, err = suite.bankMsgServer.Send(sdk.WrapSDKContext(suite.Ctx),
				tc.bankMsg(fullDenom),
			)

			if tc.expectPass {
				suite.Require().NoError(err, "test: %v", tc.desc)
			} else {
				suite.Require().Error(err, "test: %v", tc.desc)
			}
			// }
		})
	}
}
