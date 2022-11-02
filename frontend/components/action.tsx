import { DeleteIcon } from "@chakra-ui/icons";
import {
  Box,
  Button,
  Flex,
  Heading,
  Spacer,
  Table,
  TableContainer,
  Tbody,
  Td,
  Text,
  Tr,
} from "@chakra-ui/react";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import dynamic from "next/dynamic";
const ReactJson = dynamic(import("react-json-view"), { ssr: false });

const Action = ({
  msg,
  deleteAction,
}: {
  msg: ExecuteMsg;
  deleteAction?: () => void;
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
          {deleteAction && (
            <Button variant="ghost" onClick={deleteAction}>
              <DeleteIcon w={3} h={3} />
            </Button>
          )}
        </Flex>

        <Table variant="simple" size="sm">
          <Tbody>
            {kvs.map(([k, v], i) => (
              <Tr key={i}>
                <Td width="20%">
                  <Text as="b">{k}</Text>
                </Td>
                {/* @ts-ignore */}
                <Td>
                  {typeof v === "object" ? (
                    <ReactJson
                      name={null}
                      src={v || {}}
                      enableClipboard={false}
                    />
                  ) : (
                    `${v}`
                  )}
                </Td>
              </Tr>
            ))}
          </Tbody>
        </Table>
      </TableContainer>
    </Box>
  );
};

export default Action;
