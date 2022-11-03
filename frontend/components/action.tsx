import { DeleteIcon } from "@chakra-ui/icons";
import { Box, Button, Flex, Heading, Spacer } from "@chakra-ui/react";
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

      <ReactJson
        name={null}
        // @ts-ignore
        src={msg[msgType]}
        enableClipboard={false}
      />
    </Box>
  );
};

export default Action;
