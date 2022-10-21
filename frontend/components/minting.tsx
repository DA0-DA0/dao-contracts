import { AddIcon } from "@chakra-ui/icons";
import {
  Box,
  Button,
  Divider,
  FormControl,
  FormLabel,
  Heading,
  Input,
  Table,
  TableContainer,
  Tbody,
  Td,
  Th,
  Tr,
  VStack,
} from "@chakra-ui/react";
import {
  ExecuteMsg,
  Uint128,
} from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import { useForm } from "react-hook-form";
import { useDenom, useMintAllowances } from "../api/tokenfactoryIssuer";

const Minting = () => {
  const { data: denomRes } = useDenom();
  return (
    <Box>
      <Allowances></Allowances>
      <VStack>
        <SetMinterForm denom={denomRes?.denom || ""}></SetMinterForm>
        <MintForm denom={denomRes?.denom || ""}></MintForm>
      </VStack>
    </Box>
  );
};

const Allowances = () => {
  const { data: mintAllowancesRes } = useMintAllowances();
  return (
    <>
      <Heading my="10" as="h2" size="lg">
        Minting
      </Heading>
      <Heading my="5" as="h3" size="md">
        Allowances
      </Heading>
      <TableContainer>
        <Table variant="simple">
          <Tbody>
            <Tr>
              <Th>address</Th>
              <Th>allowance</Th>
            </Tr>
            {mintAllowancesRes?.allowances.map((allowance) => {
              return (
                <Tr key={"mint_allowance_" + allowance.address}>
                  <Td>{allowance.address}</Td>
                  <Td>{allowance.allowance}</Td>
                </Tr>
              );
            })}
          </Tbody>
        </Table>
      </TableContainer>
    </>
  );
};
export const SetMinterForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  const {
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<{ address: string; allowance: Uint128 }>();

  const onSubmit = async (values: { address: string; allowance: Uint128 }) => {
    onSubmitForm({
      set_minter: values,
    });
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
        Set Allowances
      </Heading>
      <Divider />
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormControl isRequired my="5">
          <FormLabel>Mint allowance</FormLabel>
          <Input
            type="number"
            id="allowance"
            disabled={isSubmitting}
            {...register("allowance")}
          ></Input>
        </FormControl>
        <FormControl isRequired my="5">
          <FormLabel>address</FormLabel>
          <Input
            type="text"
            id="address"
            disabled={isSubmitting}
            {...register("address")}
          />
        </FormControl>

        <Button variant="outline" type="submit" isLoading={isSubmitting}>
          <AddIcon w={3} h={3} mr={3} /> Add action
        </Button>
      </form>
    </Box>
  );
};

export const MintForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  const { mutate } = useMintAllowances();
  const {
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<{ to_address: string; amount: Uint128 }>();
  const onSubmit = async (values: { to_address: string; amount: Uint128 }) => {
    onSubmitForm({
      mint: values,
    });
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
        Mint
      </Heading>
      <Divider />
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormControl isRequired my="5">
          <FormLabel>mint amount</FormLabel>
          <Input
            type="number"
            id="amount"
            disabled={isSubmitting}
            {...register("amount")}
          ></Input>
        </FormControl>
        <FormControl isRequired my="5">
          <FormLabel>to address</FormLabel>
          <Input
            type="text"
            id="toAddress"
            disabled={isSubmitting}
            {...register("to_address")}
          />
        </FormControl>
        <Button variant="outline" type="submit" isLoading={isSubmitting}>
          <AddIcon w={3} h={3} mr={3} /> Add action
        </Button>
      </form>
    </Box>
  );
};

export default Minting;
