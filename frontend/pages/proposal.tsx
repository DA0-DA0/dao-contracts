import { DeleteIcon } from "@chakra-ui/icons";
import {
  Box,
  Button,
  Center,
  Flex,
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
  VStack,
} from "@chakra-ui/react";
import { Select, useStateManager } from "chakra-react-select";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import type { NextPage } from "next";
import { useState } from "react";
import { propose } from "../api/multisig";
import { BlacklistForm, SetBlacklisterForm } from "../components/blacklisting";
import { BurnForm, SetBurnerForm } from "../components/burning";
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
                <Td>
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
    options: [
      option("set_minter"),
      option("mint"),
      option("set_burner"),
      option("burn"),
      option("set_blacklister"),
      option("blacklist"),
    ],
  });

  return (
    <Center my="10" minWidth="container.xl">
      <VStack
        maxW="container.xl"
        minW="container.md"
        spacing={10}
        align="stretch"
      >
        <Heading>New Proposal</Heading>

        <Box>
          <Input
            type="text"
            my="2"
            placeholder="Title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
          <Textarea
            my="2"
            placeholder="description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
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

        <Box>
          <Box>
            <Select {...stateMgr} />
          </Box>

          <AddAction
            addAction={addAction}
            actionType={
              (!(stateMgr.value instanceof Array) && stateMgr.value?.value) ||
              ""
            }
          />
        </Box>
        <Button
          color="teal"
          variant="outline"
          onClick={async () => {
            const contract_addr = getContractAddr("tokenfactory-issuer");
            const cosmosMsgs = actions.map((action) => {
              const msg = Buffer.from(JSON.stringify(action)).toString(
                "base64"
              );
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
          }}
        >
          Submit Proposal
        </Button>
      </VStack>
    </Center>
  );
};

const AddAction = ({
  addAction,
  actionType,
}: {
  addAction: (action: ExecuteMsg) => void;
  actionType: string;
}) => {
  return (
    {
      set_minter: <SetMinterForm onSubmitForm={addAction} />,
      mint: <MintForm onSubmitForm={addAction} />,
      set_burner: <SetBurnerForm onSubmitForm={addAction} />,
      burn: <BurnForm onSubmitForm={addAction} />,
      set_blacklister: <SetBlacklisterForm onSubmitForm={addAction} />,
      blacklist: <BlacklistForm onSubmitForm={addAction} />,
    }[actionType] || <></>
  );
};

export default Home;
