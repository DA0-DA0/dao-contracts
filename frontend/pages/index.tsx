import {
  Box,
  Center,
  Divider,
  Heading,
  Skeleton,
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
  VStack,
} from "@chakra-ui/react";
import type { NextPage } from "next";
import Head from "next/head";
import { useBalance, useDenomMetadata } from "../api/bank";
import { useAddress } from "../api/keplr";
import { useDenom, useOwner } from "../api/tokenfactoryIssuer";
import Blacklistng from "../components/blacklisting";
import Burning from "../components/burning";
import Minting from "../components/minting";

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

  const metadata = denomMetadata?.metadata;

  return (
    <Center my="10" minWidth="container.md">
      <Head>
        <title>Tokenfactory Issuer UI</title>
        <meta name="description" content="Tokenfactory Issuer UI" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <VStack maxW="container.md" spacing={10} align="stretch">
        <Box>
          <Heading>Dashboard</Heading>
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
                    <Td>denom</Td>
                    <Td>{denomRes?.denom}</Td>
                  </Tr>
                  <Tr>
                    <Td>issuer contract owner</Td>
                    <Td>{ownerRes?.address}</Td>
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
      </VStack>
    </Center>
  );
};

export default Home;
