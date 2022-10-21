import { DeleteIcon } from "@chakra-ui/icons";
import {
  Box,
  Button,
  Center,
  Flex,
  Heading,
  Spacer,
  Table,
  TableContainer,
  Tbody,
  Td,
  Text,
  Tr,
  VStack,
} from "@chakra-ui/react";
import { Select, useStateManager } from "chakra-react-select";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import type { NextPage } from "next";
import { useState } from "react";
import { MintForm, SetMinterForm } from "../components/minting";

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
                <Td>{v}</Td>
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

  const deleteActionAt = (index: number) => () => {
    setActions((prev) => {
      let updateActions = [...prev];
      updateActions.splice(index, 1);
      return updateActions;
    });
  };

  const stateMgr = useStateManager({
    colorScheme: "purple",
    options: [
      {
        label: "Set minter",
        value: "set_minter",
      },
      {
        label: "Mint",
        value: "mint",
      },
    ],
  });

  return (
    <Center my="10" minWidth="container.xl">
      <VStack maxW="container.xl" spacing={10} align="stretch">
        <Heading>New Proposal</Heading>

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
              undefined
            }
          />
        </Box>
      </VStack>
    </Center>
  );
};

const AddAction = ({
  addAction,
  actionType,
}: {
  addAction: (action: ExecuteMsg) => void;
  actionType: string | undefined;
}) => {
  switch (actionType) {
    case "set_minter":
      return <SetMinterForm onSubmitForm={addAction} />;
    case "mint":
      return <MintForm onSubmitForm={addAction} />;
    default:
      return <></>;
  }
};

export default Home;
