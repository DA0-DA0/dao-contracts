import {
  Alert,
  AlertIcon,
  AlertTitle,
  Badge,
  Box,
  Center,
  Heading,
  Skeleton,
  Text,
  VStack,
} from "@chakra-ui/react";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import type { NextPage } from "next";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import { useProposal } from "../../api/multisig";
import Action from "../../components/action";

const Proposal: NextPage = () => {
  const router = useRouter();
  const { id } = router.query;

  const { data, error, mutate } = useProposal(
    // @ts-ignore
    parseInt(id),
    typeof id === "undefined"
  );

  useEffect(() => {
    if (error) {
      console.error(error);
    }
  }, [error]);

  useEffect(() => {
    mutate();
  }, [id, mutate]);

  const [navLang, setNavLang] = useState<string>();
  const actions = data?.msgs?.map((m: { wasm: { execute: { msg: string } } }) =>
    JSON.parse(Buffer.from(m.wasm.execute.msg, "base64").toString())
  );

  const statusBadgeColorMap: Record<string, string> = {
    pending: "yellow",
    open: "blue",
    rejected: "red",
    passed: "green",
    executed: "purple",
  };
  useEffect(() => {
    if (navigator.languages && navigator.languages.length) {
      setNavLang(navigator.languages[0]);
    } else {
      setNavLang(navigator.language || "en");
    }
  }, []);

  const expireTimeFormat = () => {
    const timeStr = data?.expires?.at_time;
    if (!timeStr) {
      return "";
    }
    const timeStrMillis = timeStr.slice(0, timeStr.length - 6);
    return new Intl.DateTimeFormat(navLang, {
      dateStyle: "full",
      timeStyle: "long",
    }).format(timeStrMillis);
  };

  return (
    <>
      {error ? (
        <Alert
          status="error"
          variant="subtle"
          flexDirection="column"
          alignItems="center"
          justifyContent="center"
          textAlign="center"
          height="200px"
        >
          <AlertIcon boxSize="40px" mr={0} />
          <AlertTitle mt={4} mb={1} fontSize="lg">
            Error fetching proposal
          </AlertTitle>
        </Alert>
      ) : (
        <Center my="10" minWidth="container.xl">
          <VStack
            maxW="container.xl"
            minW="container.md"
            spacing={10}
            align="stretch"
          >
            <Skeleton isLoaded={data}>
              <Heading>{data?.title || "..."}</Heading>
              <Box my="2">
                <Text as="b">status: </Text>
                <Badge
                  colorScheme={statusBadgeColorMap[`${data?.status}`] || "gray"}
                >
                  {data?.status || "..."}
                </Badge>
              </Box>

              <Box my="2">
                <Text as="b">expires at: </Text>
                <Text as="samp" fontSize="sm">
                  {navLang && expireTimeFormat()}
                </Text>
              </Box>

              <Box
                p="3"
                my="5"
                border="dotted"
                borderRadius="md"
                borderColor="gray.200"
              >
                <Text>{data?.description || "..."}</Text>
              </Box>

              <VStack>
                {actions?.map((action: ExecuteMsg, i: number) => (
                  <Action key={i} msg={action}></Action>
                ))}
              </VStack>
            </Skeleton>
          </VStack>
        </Center>
      )}
    </>
  );
};

export default Proposal;
