import {
  AddIcon,
  ArrowBackIcon,
  ArrowForwardIcon,
  LinkIcon,
} from "@chakra-ui/icons";
import {
  Alert,
  AlertIcon,
  AlertTitle,
  Badge,
  Button,
  Center,
  Flex,
  Heading,
  Skeleton,
  Spacer,
  Table,
  TableContainer,
  Tbody,
  Td,
  Th,
  Tr,
  VStack,
} from "@chakra-ui/react";

import type { NextPage } from "next";
import Link from "next/link";
import { useEffect, useState } from "react";
import { useReverseProposals } from "../api/multisig";

const Proposals: NextPage = () => {
  const [startBefore, setStartBefore] = useState<string | undefined>(undefined);
  const [startBeforeHistory, setStartBeforeHistory] = useState<
    (string | undefined)[]
  >([]);

  const {
    data: proposals,
    error: proposalsError,
    mutate,
  } = useReverseProposals(startBefore, undefined);

  useEffect(() => {
    mutate();
  }, [startBefore, mutate, proposals?.proposals]);

  useEffect(() => {
    if (proposalsError) {
      console.error(proposalsError);
    }
  }, [proposalsError]);

  const nextStartBefore =
    proposals?.proposals[proposals?.proposals?.length - 1]?.id;

  const statusBadgeColorMap: Record<string, string> = {
    pending: "yellow",
    open: "blue",
    rejected: "red",
    passed: "green",
    executed: "purple",
  };

  return (
    <>
      {proposalsError ? (
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
            Error fetching proposals
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
            <Flex alignItems="flex-end">
              <Heading>Proposals</Heading>
              <Link href="/proposal">
                <Button mx="5" variant="outline" size="sm">
                  <AddIcon mr="2" /> New
                </Button>
              </Link>
            </Flex>

            <Skeleton isLoaded={proposals}>
              <TableContainer>
                <Table variant="simple">
                  <Tbody>
                    <Tr>
                      <Th>id</Th>
                      <Th>title</Th>
                      <Th>status</Th>
                      <Th>link</Th>
                    </Tr>
                    {proposals &&
                      proposals?.proposals?.map(
                        (
                          {
                            id,
                            title,
                            status,
                          }: { id: number; title: string; status: string },
                          i: number
                        ) => {
                          return (
                            <Tr key={i}>
                              <Td>{id}</Td>
                              <Td>{title}</Td>
                              <Td>
                                <Badge
                                  colorScheme={statusBadgeColorMap[status]}
                                >
                                  {status}
                                </Badge>
                              </Td>
                              <Td>
                                <Link href={`/proposal/${id}`}>
                                  <Button variant="ghost" size="sm">
                                    <LinkIcon mx="2px" />
                                  </Button>
                                </Link>
                              </Td>
                            </Tr>
                          );
                        }
                      )}
                  </Tbody>
                </Table>
              </TableContainer>

              <Flex mt="10">
                <Spacer />
                <Button
                  variant="outline"
                  disabled={startBeforeHistory.length === 0}
                  isLoading={!proposals}
                  onClick={() => {
                    setStartBeforeHistory((hist) => {
                      const newHist = [...hist];
                      setStartBefore(newHist.pop());
                      return newHist;
                    });
                  }}
                >
                  <ArrowBackIcon />
                </Button>
                <Button
                  variant="outline"
                  isDisabled={nextStartBefore === 1}
                  isLoading={!proposals}
                  onClick={() => {
                    if (proposals?.proposals?.length === 0) {
                      mutate();
                      return;
                    }

                    setStartBeforeHistory((hist) => [...hist, startBefore]);
                    setStartBefore(nextStartBefore);
                  }}
                >
                  <ArrowForwardIcon />
                </Button>
                <Spacer />
              </Flex>
            </Skeleton>
          </VStack>
        </Center>
      )}
    </>
  );
};

export default Proposals;
