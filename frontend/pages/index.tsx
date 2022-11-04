import { ArrowBackIcon, ArrowForwardIcon } from "@chakra-ui/icons";
import {
  Alert,
  AlertIcon,
  AlertTitle,
  Box,
  Button,
  Center,
  Divider,
  Flex,
  Heading,
  Link,
  Skeleton,
  Spacer,
  Tab,
  Table,
  TableContainer,
  TabList,
  TabPanel,
  TabPanels,
  Tabs,
  Tbody,
  Td,
  Th,
  Thead,
  Tr,
  useToast,
  VStack,
} from "@chakra-ui/react";
import type { NextPage } from "next";
import dynamic from "next/dynamic";
import Head from "next/head";

import { useEffect, useState } from "react";
import { useBalance, useDenomMetadata, useSupply } from "../api/bank";
import { useAddress } from "../api/keplr";
import { useThreshold, useVoters } from "../api/multisig";
import { useDenom, useIsFrozen, useOwner } from "../api/tokenfactoryIssuer";
import Blacklistng from "../components/blacklisting";
import Burning from "../components/burning";
import Minting from "../components/minting";
const ReactJson = dynamic(import("react-json-view"), { ssr: false });

const Home: NextPage = () => {
  const { data: denomRes, error: denomErr } = useDenom();
  const { data: ownerRes, error: ownerErr } = useOwner();
  const { data: address, error: addressErr } = useAddress();
  const { data: balance, error: balanceErr } = useBalance(
    address || "",
    denomRes?.denom || ""
  );

  const { data: denomMetadata, error: denomMetadataErr } = useDenomMetadata(
    denomRes?.denom || ""
  );

  const { data: supply, error: supplyErr } = useSupply(denomRes?.denom || "");
  const { data: isFrozen, error: isFrozenErr } = useIsFrozen();

  const { data: threshold, error: thresholdErr } = useThreshold();

  const metadata = denomMetadata?.metadata;

  const errors = [
    denomErr,
    ownerErr,
    addressErr,
    balanceErr,
    denomMetadataErr,
    supplyErr,
    isFrozenErr,
    thresholdErr,
  ];

  useEffect(() => {
    if (errors.some((e) => typeof e !== "undefined")) {
      console.error(errors.filter((e) => typeof e !== "undefined"));
    }

    // eslint-disable-next-line
  }, errors);

  return (
    <Center my="10" minWidth="container.md">
      <Head>
        <title>Tokenfactory Issuer UI</title>
        <meta name="description" content="Tokenfactory Issuer UI" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <VStack maxW="container.md" spacing={10} align="stretch">
        <Heading>Dashboard</Heading>
        <Box>
          <Heading size="md">Token & Issuer Info</Heading>
          <Skeleton
            isLoaded={
              typeof denomRes !== "undefined" &&
              typeof denomMetadata !== "undefined"
            }
          >
            <TableContainer maxW="container.md" py="5">
              <Table variant="simple">
                <Tbody>
                  <Tr>
                    <Td>base denom</Td>
                    <Td>{denomRes?.denom}</Td>
                  </Tr>
                  <Tr>
                    <Td>issuer contract owner</Td>
                    <Td>{ownerRes?.address}</Td>
                  </Tr>
                  <Tr>
                    <Td>frozen</Td>
                    <Td>{`${isFrozen?.is_frozen}`}</Td>
                  </Tr>
                  <Tr>
                    <Td>description</Td>
                    <Td>{metadata?.description}</Td>
                  </Tr>
                  <Tr>
                    <Td>display</Td>
                    <Td>{metadata?.display}</Td>
                  </Tr>
                  <Tr>
                    <Td>name</Td>
                    <Td>{metadata?.name}</Td>
                  </Tr>
                  <Tr>
                    <Td>symbol</Td>
                    <Td>{metadata?.symbol}</Td>
                  </Tr>
                  <Tr>
                    <Td>supply</Td>
                    <Td>{supply?.amount?.amount}</Td>
                  </Tr>
                  <Tr>
                    <Td>my token ({address?.substring(0, 10)}...)</Td>
                    <Td>{balance?.balance?.amount}</Td>
                  </Tr>
                </Tbody>
              </Table>
            </TableContainer>

            <Divider my="5" />
            <Heading size="sm">Denom Units</Heading>
            <TableContainer maxW="container.md" py="5">
              <Table variant="simple" size="sm">
                <Thead>
                  <Tr>
                    <Th>denom</Th>
                    <Th>exponent</Th>
                    <Th>aliases</Th>
                  </Tr>
                </Thead>
                <Tbody>
                  {metadata?.denomUnits.map((d, i) => {
                    return (
                      <Tr key={i}>
                        <Td>{d.denom}</Td>
                        <Td>{d.exponent}</Td>
                        <Td>{d.aliases.join(", ")}</Td>
                      </Tr>
                    );
                  })}
                </Tbody>
              </Table>
            </TableContainer>
          </Skeleton>
        </Box>
        <Divider my="5" />
        <Box>
          <Heading size="md">Multisig</Heading>
          <Box py="5">
            <Heading size="sm">Threshold</Heading>
            <Box py="3">
              <ReactJson src={threshold} name={null} enableClipboard={false} />
              <Box fontSize="sm" fontStyle="italic">
                Note that weight matters.{" "}
                <Link
                  target="_blank"
                  href="https://docs.rs/cw-utils/latest/cw_utils/enum.ThresholdResponse.html#variants"
                >
                  <Button size="sm" variant="link">
                    More info
                  </Button>
                </Link>
              </Box>
            </Box>
          </Box>
          <Box py="5">
            <Heading size="sm">Voters</Heading>
            <Voters />
          </Box>
        </Box>
        {process.env.NEXT_PUBLIC_TOGGLE_ALLOWANCE && (
          <Tabs variant="line" colorScheme="blackAlpha">
            <TabList>
              <Tab>Minting</Tab>
              <Tab>Blacklisting</Tab>
              <Tab>Burning</Tab>
            </TabList>
            <TabPanels>
              <TabPanel>
                <Minting></Minting>
              </TabPanel>
              <TabPanel>
                <Blacklistng></Blacklistng>
              </TabPanel>
              <TabPanel>
                <Burning></Burning>
              </TabPanel>
            </TabPanels>
          </Tabs>
        )}
      </VStack>
    </Center>
  );
};

const Voters = () => {
  const toast = useToast();
  const [startAfter, setStartAfter] = useState<string | undefined>(undefined);
  const [startAfterHistory, setStartAfterHistory] = useState<
    (string | undefined)[]
  >([]);

  const { data: voters, error, mutate } = useVoters(startAfter, undefined);

  useEffect(() => {
    if (voters?.voters?.length === 0) {
      toast({
        title: "No more votes currently available",
        description:
          "We've reached the end of vote list. Click `->` again to check if there is any update.",
        status: "info",
        isClosable: true,
      });
    }
  }, [voters, toast]);

  // update votes when startAfter changes
  useEffect(() => {
    mutate();
  }, [startAfter, mutate]);

  useEffect(() => {
    if (error) {
      console.error(error);
    }
  }, [error]);

  return (
    <Box>
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
        <Skeleton isLoaded={!!voters}>
          <TableContainer maxW="container.md" py="5">
            <Table variant="simple" size="md">
              <Thead>
                <Tr>
                  <Th>address</Th>
                  <Th>weight</Th>
                </Tr>
              </Thead>
              <Tbody>
                {voters?.voters.map((v, i) => {
                  return (
                    <Tr key={i}>
                      <Td>{v.addr}</Td>
                      <Td>{v.weight}</Td>
                    </Tr>
                  );
                })}
              </Tbody>
            </Table>
          </TableContainer>

          <Flex mt="10">
            <Spacer />
            <Button
              variant="outline"
              disabled={startAfterHistory.length === 0}
              isLoading={!voters}
              onClick={() => {
                setStartAfterHistory((hist) => {
                  const newHist = [...hist];
                  setStartAfter(newHist.pop());
                  return newHist;
                });
              }}
            >
              <ArrowBackIcon />
            </Button>

            <Button
              variant="outline"
              isLoading={!voters}
              onClick={() => {
                if (voters?.voters.length === 0) {
                  mutate();
                  return;
                }

                const nextStartAfter =
                  voters?.voters[voters?.voters?.length - 1]?.addr;

                setStartAfterHistory((hist) => [...hist, startAfter]);
                setStartAfter(nextStartAfter);
              }}
            >
              <ArrowForwardIcon />
            </Button>
            <Spacer />
          </Flex>
        </Skeleton>
      )}
    </Box>
  );
};

export default Home;
