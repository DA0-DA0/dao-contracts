import { Center, Heading, VStack } from "@chakra-ui/react";
import type { NextPage } from "next";
import { useRouter } from "next/router";

const Proposal: NextPage = () => {
  const router = useRouter();
  const { id } = router.query;

  return (
    <>
      <Center my="10" minWidth="container.xl">
        <VStack
          maxW="container.xl"
          minW="container.md"
          spacing={10}
          align="stretch"
        >
          <Heading>Proposal {id}</Heading>
        </VStack>
      </Center>
    </>
  );
};

export default Proposal;
