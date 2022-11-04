import { InfoIcon } from "@chakra-ui/icons";
import {
  Box,
  Button,
  Center,
  Drawer,
  DrawerBody,
  DrawerCloseButton,
  DrawerContent,
  DrawerHeader,
  DrawerOverlay,
  FormControl,
  FormLabel,
  Heading,
  Input,
  Textarea,
  useBoolean,
  useDisclosure,
  useToast,
  VStack,
} from "@chakra-ui/react";
import { Select, useStateManager } from "chakra-react-select";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import type { NextPage } from "next";
import { useRouter } from "next/router";
import React, { useRef, useState } from "react";
import { propose } from "../api/multisig";
import Action from "../components/action";
import { BlacklistForm, SetBlacklisterForm } from "../components/blacklisting";
import { BurnForm, SetBurnerForm } from "../components/burning";
import { FreezeForm, SetFreezerForm } from "../components/freezing";
import * as group from "../components/group";
import { MintForm, SetMinterForm } from "../components/minting";
import { SetDenomMetadataForm } from "../components/setDenomMetadata";
import { getContractAddr } from "../lib/beakerState";

const Proposal: NextPage = () => {
  const router = useRouter();
  const toast = useToast();
  const [isLoading, setIsLoading] = useBoolean();
  const [actions, setActions] = useState<ExecuteMsg[]>([]);

  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");

  const { isOpen, onOpen, onClose } = useDisclosure();
  const submitBtnRef = useRef(null);
  const selectActionRef = useRef<HTMLElement>(null);

  const addAction = (action: ExecuteMsg) => {
    setActions((prev) => [...prev, action]);
    selectActionRef?.current?.focus();
  };

  const deleteActionAt = (index: number) => () => {
    setActions((prev) => {
      let updateActions = [...prev];
      updateActions.splice(index, 1);
      return updateActions;
    });
  };

  const option = (value: string) => ({
    label: value,
    value,
  });
  const stateMgr = useStateManager({
    colorScheme: "purple",
    options: Object.keys(actionFormMap).map(option),
  });

  const submitProposal = async () => {
    const tokenfactory_issuer_addr = getContractAddr("tokenfactory-issuer");
    const cw4_group_addr = getContractAddr("cw4-group");

    const groupMsgs = ["update_members"];

    const cosmosMsgs = actions.map((action) => {
      const msg = Buffer.from(JSON.stringify(action)).toString("base64");
      const msgType = Object.keys(action)[0];

      let contract_addr = tokenfactory_issuer_addr;
      if (groupMsgs.some((m: string) => m === msgType)) {
        contract_addr = cw4_group_addr;
      }

      // wrap in a cosmwasm msg structure
      return {
        wasm: {
          execute: {
            contract_addr,
            msg,
            funds: [],
          },
        },
      };
    });

    setIsLoading.on();
    const proposal = await propose(title, description, cosmosMsgs);
    setIsLoading.off();

    const proposalId = proposal?.logs[0]?.events
      .find((e) => e.type === "wasm")
      ?.attributes?.find((attr) => attr.key === "proposal_id")?.value;

    router.push(`/proposal/${proposalId}`);
  };

  return (
    <>
      <Center my="10" minWidth="container.xl">
        <form
          onSubmit={async (e) => {
            e.preventDefault();
            try {
              await submitProposal();
            } catch (error) {
              setIsLoading.off();
              console.error(error);

              toast({
                title: "Error submiting proposal",
                isClosable: true,
                description: `${error}`,
                status: "error",
              });
            }
          }}
        >
          <VStack
            maxW="container.xl"
            minW="container.md"
            spacing={10}
            align="stretch"
          >
            <Heading>New Proposal</Heading>

            <Box>
              <FormControl my="2" isDisabled={isLoading}>
                <FormLabel>Title</FormLabel>
                <Input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                />
              </FormControl>
              <FormControl my="2" isDisabled={isLoading}>
                <FormLabel>Description</FormLabel>
                <Textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </FormControl>
            </Box>
            <VStack opacity={isLoading ? "0.5" : 1}>
              {actions.map((action, i) => (
                <Action
                  key={i}
                  msg={action}
                  deleteAction={deleteActionAt(i)}
                ></Action>
              ))}
            </VStack>

            <Button onClick={onOpen} isDisabled={isLoading} variant="outline">
              Add Action
            </Button>

            <Button
              color="teal"
              variant="outline"
              type="submit"
              ref={submitBtnRef}
              isLoading={isLoading}
            >
              Submit Proposal
            </Button>
          </VStack>
        </form>
      </Center>

      {/* hack for dev mode chakra ui portal problem: https://github.com/chakra-ui/chakra-ui/issues/6297 */}
      {(isOpen || process.env.NODE_ENV === "production") && (
        <Drawer
          isOpen={isOpen}
          placement="top"
          onClose={onClose}
          initialFocusRef={selectActionRef}
          finalFocusRef={submitBtnRef}
        >
          <DrawerOverlay />
          <DrawerContent>
            <DrawerCloseButton />
            <DrawerHeader>Add Action</DrawerHeader>

            <DrawerBody>
              <Box>
                <Select
                  // @ts-ignore
                  ref={selectActionRef}
                  placeholder="Select action type..."
                  {...stateMgr}
                />
              </Box>

              <AddAction
                addAction={addAction}
                actionType={
                  (!(stateMgr.value instanceof Array) &&
                    stateMgr.value?.value) ||
                  ""
                }
              />
            </DrawerBody>
          </DrawerContent>
        </Drawer>
      )}
    </>
  );
};

// register action components
const actionFormMap: Record<
  string,
  React.FC<{ onSubmitForm: (msg: ExecuteMsg) => void }> | undefined
> = {
  "issuer::set_minter": SetMinterForm,
  "issuer::mint": MintForm,
  "issuer::set_burner": SetBurnerForm,
  "issuer::burn": BurnForm,
  "issuer::set_blacklister": SetBlacklisterForm,
  "issuer::blacklist": BlacklistForm,
  "issuer::set_freezer": SetFreezerForm,
  "issuer::freeze": FreezeForm,
  "issuer::set_denom_metadata": SetDenomMetadataForm,
  "group::update_members": group.UpdateMembers,
};

const AddAction = ({
  addAction,
  actionType,
}: {
  addAction: (action: ExecuteMsg) => void;
  actionType: string;
}) => {
  const FormComponent = actionFormMap[actionType];
  return typeof FormComponent !== "undefined" ? (
    <FormComponent onSubmitForm={addAction} />
  ) : (
    <Center py="60" color="grey">
      <InfoIcon mr="2" />
      Please select action type to add.
    </Center>
  );
};

export default Proposal;
