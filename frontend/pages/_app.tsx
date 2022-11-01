import "../styles/globals.css";

import { Button, Center, ChakraProvider, HStack } from "@chakra-ui/react";
import { AppProps } from "next/app";
import Link from "next/link";

function MyApp({ Component, pageProps }: AppProps) {
  return (
    <ChakraProvider>
      <Center width="full" borderBottom="2px" py="5 ">
        <HStack minW="container.md" spacing="5">
          <Link href="/">
            <Button variant="link" colorScheme="black">
              dashboard
            </Button>
          </Link>
          <Link href="/proposals">
            <Button variant="link" colorScheme="black">
              proposals
            </Button>
          </Link>
        </HStack>
      </Center>
      <Component {...pageProps} />
    </ChakraProvider>
  );
}

export default MyApp;
