import { ArrowBackIcon, ArrowForwardIcon, LinkIcon } from "@chakra-ui/icons";
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
  useToast,
  VStack,
} from "@chakra-ui/react";

import type { NextPage } from "next";
import Link from "next/link";
import { useEffect, useState } from "react";
import { useReverseProposals } from "../api/multisig";

const Proposals: NextPage = () => {
  const toast = useToast();
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
            <Heading>Proposals</Heading>
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

const ProposalList = () => {
  return <></>;
  // const toast = useToast();
  // const [startAfter, setStartAfter] = useState<string | undefined>(undefined);
  // const [startAfterHistory, setStartAfterHistory] = useState<
  //   (string | undefined)[]
  // >([]);
  // useEffect(() => {
  //   if (currentVotes?.votes?.length === 0) {
  //     toast({
  //       title: "No more votes currently available",
  //       description:
  //         "We've reached the end of vote list. Click `->` again to check if there is any update.",
  //       status: "info",
  //       isClosable: true,
  //     });
  //   }
  // }, [currentVotes, toast]);
  // // update votes when startAfter changes
  // useEffect(() => {
  //   mutateCurrentVotes();
  // }, [startAfter, mutateCurrentVotes]);
  // useEffect(() => {
  //   console.error(error);
  // }, [error]);
  // return (
  //   <Box my="10">
  //     <Heading size="md" my="5">
  //       Votes
  //     </Heading>
  //     {error ? (
  //       <Alert
  //         status="error"
  //         variant="subtle"
  //         flexDirection="column"
  //         alignItems="center"
  //         justifyContent="center"
  //         textAlign="center"
  //         height="200px"
  //       >
  //         <AlertIcon boxSize="40px" mr={0} />
  //         <AlertTitle mt={4} mb={1} fontSize="lg">
  //           Error fetching votes
  //         </AlertTitle>
  //       </Alert>
  //     ) : (
  //       <Skeleton isLoaded={currentVotes}>
  //         <TableContainer>
  //           <Table variant="simple">
  //             <Tbody>
  //               <Tr>
  //                 <Th>voter</Th>
  //                 <Th>vote</Th>
  //                 <Th>weight</Th>
  //               </Tr>
  //               {currentVotes?.votes.map(
  //                 (
  //                   voteInfo: { voter: string; vote: string; weight: number },
  //                   i: number
  //                 ) => {
  //                   return (
  //                     <Tr key={i}>
  //                       <Td>{voteInfo.voter}</Td>
  //                       <Td>
  //                         <VoteBadge vote={voteInfo.vote} />
  //                       </Td>
  //                       <Td>{voteInfo.weight}</Td>
  //                     </Tr>
  //                   );
  //                 }
  //               )}
  //             </Tbody>
  //           </Table>
  //         </TableContainer>
  //         <Flex mt="10">
  //           <Spacer />
  //           <Button
  //             variant="outline"
  //             disabled={startAfterHistory.length === 0}
  //             isLoading={!currentVotes}
  //             onClick={() => {
  //               setStartAfterHistory((hist) => {
  //                 const newHist = [...hist];
  //                 setStartAfter(newHist.pop());
  //                 return newHist;
  //               });
  //             }}
  //           >
  //             <ArrowBackIcon />
  //           </Button>
  //           <Button
  //             variant="outline"
  //             isLoading={!currentVotes}
  //             onClick={() => {
  //               if (currentVotes?.votes.length === 0) {
  //                 mutateCurrentVotes();
  //                 return;
  //               }
  //               const nextStartAfter =
  //                 currentVotes?.votes[currentVotes?.votes?.length - 1]?.voter;
  //               setStartAfterHistory((hist) => [...hist, startAfter]);
  //               setStartAfter(nextStartAfter);
  //             }}
  //           >
  //             <ArrowForwardIcon />
  //           </Button>
  //           <Spacer />
  //         </Flex>
  //       </Skeleton>
  //     )}
  //   </Box>
  // );
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

export default Proposals;
