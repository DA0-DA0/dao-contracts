import { DeleteIcon, InfoIcon } from "@chakra-ui/icons";
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
  Flex,
  FormControl,
  FormLabel,
  Heading,
  Input,
  Spacer,
  Table,
  TableContainer,
  Tbody,
  Td,
  Text,
  Textarea,
  Tr,
  useDisclosure,
  VStack,
} from "@chakra-ui/react";
import { Select, useStateManager } from "chakra-react-select";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import type { NextPage } from "next";
import React, { useRef, useState } from "react";
import { propose } from "../api/multisig";
import { BlacklistForm, SetBlacklisterForm } from "../components/blacklisting";
import { BurnForm, SetBurnerForm } from "../components/burning";
import { FreezeForm, SetFreezerForm } from "../components/freezing";
import { MintForm, SetMinterForm } from "../components/minting";
import { getContractAddr } from "../lib/beakerState";

const Action = ({
  msg,
  deleteAction,
}: {
  msg: ExecuteMsg;
  deleteAction: () => void;
}) => {
  const msgType = Object.keys(msg)[0];
  // @ts-ignore
  const kvs = Object.entries(msg[msgType]);

  return (
    <Box
      border="2px"
      borderColor="gray.200"
      borderRadius="md"
      p="9"
      minWidth="container.md"
    >
      <TableContainer>
        <Flex>
          <Box>
            <Heading mb="3" size="sm">
              {msgType}
            </Heading>
          </Box>

          <Spacer />
          <Button variant="ghost" onClick={deleteAction}>
            <DeleteIcon w={3} h={3} />
          </Button>
        </Flex>

        <Table variant="simple" size="sm">
          <Tbody>
            {kvs.map(([k, v], i) => (
              <Tr key={i}>
                <Td width="20%">
                  <Text as="b">{k}</Text>
                </Td>
                {/* @ts-ignore */}
                <Td>{`${v}`}</Td>
              </Tr>
            ))}
          </Tbody>
        </Table>
      </TableContainer>
    </Box>
  );
};

const Home: NextPage = () => {
  const [actions, setActions] = useState<ExecuteMsg[]>([]);
  const addAction = (action: ExecuteMsg) =>
    setActions((prev) => [...prev, action]);

  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");

  const { isOpen, onOpen, onClose } = useDisclosure();
  const btnRef = useRef(null);

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
    const contract_addr = getContractAddr("tokenfactory-issuer");
    const cosmosMsgs = actions.map((action) => {
      const msg = Buffer.from(JSON.stringify(action)).toString("base64");
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

    const proposal = await propose(title, description, cosmosMsgs);

    // TODO: redirect to proposal list page
    console.log(proposal.transactionHash);
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
              // TODO: handle error
              console.error(error);
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
              <FormControl my="2">
                <FormLabel>Title</FormLabel>
                <Input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                />
              </FormControl>
              <FormControl my="2">
                <FormLabel>Description</FormLabel>
                <Textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </FormControl>
            </Box>
            <VStack>
              {actions.map((action, i) => (
                <Action
                  key={i}
                  msg={action}
                  deleteAction={deleteActionAt(i)}
                ></Action>
              ))}
            </VStack>

            <Button onClick={onOpen}>Add Action</Button>

            <Button color="teal" variant="outline" type="submit">
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
          finalFocusRef={btnRef}
        >
          <DrawerOverlay />
          <DrawerContent>
            <DrawerCloseButton />
            <DrawerHeader>Add Action</DrawerHeader>

            <DrawerBody>
              <Box>
                <Select placeholder="Select action type..." {...stateMgr} />
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

const actionFormMap: Record<
  string,
  React.FC<{ onSubmitForm: (msg: ExecuteMsg) => void }> | undefined
> = {
  set_minter: SetMinterForm,
  mint: MintForm,
  set_burner: SetBurnerForm,
  burn: BurnForm,
  set_blacklister: SetBlacklisterForm,
  blacklist: BlacklistForm,
  set_freezer: SetFreezerForm,
  freeze: FreezeForm,
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

export default Home;
