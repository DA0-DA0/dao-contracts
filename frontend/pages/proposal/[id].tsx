import {
  Alert,
  AlertIcon,
  AlertTitle,
  Badge,
  Box,
  Button,
  Center,
  Heading,
  HStack,
  Skeleton,
  Table,
  TableContainer,
  Tbody,
  Td,
  Text,
  Th,
  Tr,
  useToast,
  VStack,
} from "@chakra-ui/react";

import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import type { NextPage } from "next";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import { mutate } from "swr";
import { execute, useProposal, useVotes, vote } from "../../api/multisig";
import Action from "../../components/action";

const Proposal: NextPage = () => {
  const router = useRouter();
  const { id } = router.query;
  const proposal_id = parseInt(typeof id === "string" ? id : "");
  const [isLoading, setIsLoading] = useState(false);
  const toast = useToast();

  const { data, error } = useProposal(proposal_id, typeof id === "undefined");

  async function broadcastTx<T>(f: () => Promise<T>) {
    setIsLoading(true);
    try {
      await f();
    } catch (error) {
      toast({
        title: "Error broadcasting transaction",
        isClosable: true,
        description: `${error}`,
        status: "error",
      });
    }

    setIsLoading(false);
    mutate("/cw3-flex-multisig/proposal");
    mutate("/cw3-flex-multisig/votes");
  }

  useEffect(() => {
    if (error) {
      console.error(error);
    }
  }, [error]);

  useEffect(() => {
    mutate("/cw3-flex-multisig/proposal");
    mutate("/cw3-flex-multisig/votes");
  }, [id]);

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
              <Votes proposal_id={proposal_id} />
              {/* vote */}
              {data?.status === "open" && (
                <Box my="10">
                  {/* <Heading size="md">Vote</Heading> */}
                  <HStack py="5">
                    <Button
                      color="green"
                      variant="outline"
                      isLoading={isLoading}
                      onClick={() =>
                        broadcastTx(() => vote(proposal_id, "yes"))
                      }
                    >
                      Yes
                    </Button>
                    <Button
                      color="red"
                      variant="outline"
                      isLoading={isLoading}
                      onClick={() => broadcastTx(() => vote(proposal_id, "no"))}
                    >
                      No
                    </Button>
                    <Button
                      color="crimson"
                      variant="outline"
                      isLoading={isLoading}
                      onClick={() =>
                        broadcastTx(() => vote(proposal_id, "veto"))
                      }
                    >
                      Veto
                    </Button>
                    <Button
                      color="gray"
                      variant="outline"
                      isLoading={isLoading}
                      onClick={() =>
                        broadcastTx(() => vote(proposal_id, "abstain"))
                      }
                    >
                      Abstain
                    </Button>
                  </HStack>
                </Box>
              )}
              {/* execute */}
              {data?.status === "passed" && (
                <Box my="10">
                  <HStack py="5">
                    <Button
                      color="purple"
                      variant="outline"
                      isLoading={isLoading}
                      onClick={() => {
                        broadcastTx(() => execute(proposal_id));
                      }}
                    >
                      Execute
                    </Button>
                  </HStack>
                </Box>
              )}
            </Skeleton>
          </VStack>
        </Center>
      )}
    </>
  );
};

const Votes = ({ proposal_id }: { proposal_id: number }) => {
  const toast = useToast();
  const [startAfter, setStartAfter] = useState<string | undefined>(undefined);
  const [startAfterHistory, setStartAfterHistory] = useState<
    (string | undefined)[]
  >([]);

  const {
    data: currentVotes,
    error,
    mutate: mutateCurrentVotes,
  } = useVotes(proposal_id, startAfter, 1);

  useEffect(() => {
    if (currentVotes?.votes?.length === 0) {
      toast({
        title: "No more votes currently available",
        description:
          "We've reached the end of vote list. Click `Next` to check if there is any update.",
        status: "info",
        isClosable: true,
      });
    }
  }, [currentVotes, toast]);

  // update votes when startAfter changes
  useEffect(() => {
    mutateCurrentVotes();
  }, [startAfter, mutateCurrentVotes]);

  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <Box my="10">
      <Heading size="md" my="5">
        Votes
      </Heading>

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
            Error fetching votes
          </AlertTitle>
        </Alert>
      ) : (
        <Skeleton isLoaded={currentVotes}>
          <TableContainer>
            <Table variant="simple">
              <Tbody>
                <Tr>
                  <Th>voter</Th>
                  <Th>vote</Th>
                  <Th>weight</Th>
                </Tr>
                {currentVotes?.votes.map(
                  (
                    voteInfo: { voter: string; vote: string; weight: number },
                    i: number
                  ) => {
                    return (
                      <Tr key={i}>
                        <Td>{voteInfo.voter}</Td>
                        <Td>
                          <VoteBadge vote={voteInfo.vote} />
                        </Td>
                        <Td>{voteInfo.weight}</Td>
                      </Tr>
                    );
                  }
                )}
              </Tbody>
            </Table>
          </TableContainer>

          <Button
            disabled={startAfterHistory.length === 0}
            isLoading={!currentVotes}
            onClick={() => {
              setStartAfterHistory((hist) => {
                const newHist = [...hist];
                setStartAfter(newHist.pop());
                return newHist;
              });
            }}
          >
            Prev
          </Button>

          <Button
            isLoading={!currentVotes}
            onClick={() => {
              if (currentVotes?.votes.length === 0) {
                mutateCurrentVotes();
                return;
              }

              const nextStartAfter =
                currentVotes?.votes[currentVotes?.votes?.length - 1]?.voter;

              setStartAfterHistory((hist) => [...hist, startAfter]);
              setStartAfter(nextStartAfter);
            }}
          >
            Next
          </Button>
        </Skeleton>
      )}
    </Box>
  );
};

const VoteBadge = ({ vote }: { vote: string }) => {
  const colorSchemeMap: Record<string, string> = {
    yes: "green",
    no: "red",
    veto: "orange",
    abstain: "gray",
  };

  if (colorSchemeMap[vote] !== undefined) {
    return <Badge colorScheme={colorSchemeMap[vote]}>{vote}</Badge>;
  }

  return <>{vote}</>;
};

export default Proposal;
