import {
  Box,
  Button,
  Divider,
  FormControl,
  FormHelperText,
  FormLabel,
  Heading,
  Input,
  Switch,
  Table,
  TableContainer,
  Tbody,
  Td,
  Th,
  Tr,
  VStack,
} from "@chakra-ui/react";
import { useForm } from "react-hook-form";
import {
  getTokenIssuerSigningClient,
  useBlacklistees,
  useBlacklisterAllowances,
  useDenom,
} from "../api/tokenfactoryIssuer";

const Blacklistng = () => {
  const { data: denomRes } = useDenom();
  const { data } = useBlacklistees();
  return (
    <Box>
      <Allowances></Allowances>
      <Heading my="5" as="h3" size="md">
        Blacklistees
      </Heading>
      <TableContainer>
        <Table variant="simple">
          <Tbody>
            <Tr>
              <Th>address</Th>
            </Tr>
            {(data?.blacklistees || [])
              .filter((statusInfo) => statusInfo.status)
              .map((statusInfo) => {
                return (
                  <Tr key={"blacklistee_" + statusInfo.address}>
                    <Td>{statusInfo.address}</Td>
                  </Tr>
                );
              })}
          </Tbody>
        </Table>
      </TableContainer>
      <VStack>
        <SetBlacklisterForm denom={denomRes?.denom || ""}></SetBlacklisterForm>

        <BlacklistForm denom={denomRes?.denom || ""}></BlacklistForm>
      </VStack>
    </Box>
  );
};

const Allowances = () => {
  const { data } = useBlacklisterAllowances();
  return (
    <>
      <Heading my="10" as="h2" size="lg">
        Blacklisting
      </Heading>
      <Heading my="5" as="h3" size="md">
        Blacklisters
      </Heading>
      <TableContainer>
        <Table variant="simple">
          <Tbody>
            <Tr>
              <Th>address</Th>
            </Tr>
            {data?.blacklisters
              .filter((allowance) => allowance.status)
              .map((allowance) => {
                return (
                  <Tr key={"blacklister_allowance_" + allowance.address}>
                    <Td>{allowance.address}</Td>
                  </Tr>
                );
              })}
          </Tbody>
        </Table>
      </TableContainer>
    </>
  );
};
const SetBlacklisterForm = ({ denom }: { denom: string }) => {
  const { mutate } = useBlacklisterAllowances();
  const {
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm();
  const onSubmit = async (values) => {
    const client = await getTokenIssuerSigningClient();
    await client.setBlacklister(values);
    mutate();
    reset();
  };
  return (
    <Box
      my="10"
      border="2px"
      borderColor="gray.200"
      borderRadius="md"
      p="9"
      minWidth="container.md"
    >
      <Heading my="5" as="h3" size="md">
        Set Blacklister
      </Heading>
      <Divider />
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormControl isRequired my="5">
          <FormLabel>address</FormLabel>
          <Input
            type="text"
            id="address"
            disabled={isSubmitting}
            {...register("address")}
          />
          <FormHelperText>blacklister address</FormHelperText>
        </FormControl>
        <FormControl my="5">
          <FormLabel>Blacklister status</FormLabel>
          <Switch id="status" disabled={isSubmitting} {...register("status")} />

          <FormHelperText>set if the address can blacklist</FormHelperText>
        </FormControl>
        <Button
          mt={4}
          colorScheme="teal"
          isLoading={isSubmitting}
          type="submit"
        >
          Set Blacklister
        </Button>
      </form>
    </Box>
  );
};

const BlacklistForm = ({ denom }: { denom: string }) => {
  const { mutate } = useBlacklistees();
  const {
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm();
  const onSubmit = async (values) => {
    const client = await getTokenIssuerSigningClient();
    await client.blacklist(values);
    mutate();
    reset();
  };
  return (
    <Box
      my="10"
      border="2px"
      borderColor="gray.200"
      borderRadius="md"
      p="9"
      minWidth="container.md"
    >
      <Heading my="5" as="h3" size="md">
        Set Blacklistee
      </Heading>
      <Divider />
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormControl isRequired my="5">
          <FormLabel>address</FormLabel>
          <Input
            type="text"
            id="address"
            disabled={isSubmitting}
            {...register("address")}
          />
          <FormHelperText>target address</FormHelperText>
        </FormControl>
        <FormControl my="5">
          <FormLabel>Blacklistee status</FormLabel>
          <Switch id="status" disabled={isSubmitting} {...register("status")} />

          <FormHelperText>
            set if the address blacklisted / un-blacklisted
          </FormHelperText>
        </FormControl>

        <Button
          mt={4}
          colorScheme="teal"
          isLoading={isSubmitting}
          type="submit"
        >
          Set Blacklistee
        </Button>
      </form>
    </Box>
  );
};

export default Blacklistng;
