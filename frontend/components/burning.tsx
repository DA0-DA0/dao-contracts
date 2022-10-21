import {
  Box,
  Button,
  Divider,
  FormControl,
  FormHelperText,
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
import { useForm } from "react-hook-form";
import {
  getTokenIssuerSigningClient,
  useBurnAllowances,
  useDenom,
} from "../api/tokenfactoryIssuer";

const Burning = () => {
  const { data: denomRes } = useDenom();
  return (
    <Box>
      <Allowances></Allowances>
      <VStack>
        <SetBurnerForm denom={denomRes?.denom || ""}></SetBurnerForm>
        <BurnForm denom={denomRes?.denom || ""}></BurnForm>
      </VStack>
    </Box>
  );
};

const Allowances = () => {
  const { data: burnAllowancesRes } = useBurnAllowances();
  return (
    <>
      <Heading my="10" as="h2" size="lg">
        Burning
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
            {burnAllowancesRes?.allowances.map((allowance) => {
              return (
                <Tr key={"burn_allowance_" + allowance.address}>
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
const SetBurnerForm = ({ denom }: { denom: string }) => {
  const { mutate } = useBurnAllowances();
  const {
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm();
  const onSubmit = async (values) => {
    const client = await getTokenIssuerSigningClient();
    await client.setBurner(values);
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
        Set Allowances
      </Heading>
      <Divider />
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormControl isRequired my="5">
          <FormLabel>Burn allowance</FormLabel>
          <Input
            type="number"
            id="allowance"
            disabled={isSubmitting}
            {...register("allowance")}
          ></Input>
          <FormHelperText>
            amount of `{denom}` to allow burner to burn
          </FormHelperText>
        </FormControl>
        <FormControl isRequired my="5">
          <FormLabel>address</FormLabel>
          <Input
            type="text"
            id="address"
            disabled={isSubmitting}
            {...register("address")}
          />
          <FormHelperText>burner address</FormHelperText>
        </FormControl>
        <Button
          mt={4}
          colorScheme="teal"
          isLoading={isSubmitting}
          type="submit"
        >
          Set Burner
        </Button>
      </form>
    </Box>
  );
};

const BurnForm = ({ denom }: { denom: string }) => {
  const { mutate } = useBurnAllowances();
  const {
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm();
  const onSubmit = async (values) => {
    const client = await getTokenIssuerSigningClient();
    await client.burn(values);
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
        Burn
      </Heading>
      <Divider />
      <form onSubmit={handleSubmit(onSubmit)}>
        <FormControl isRequired my="5">
          <FormLabel>burn amount</FormLabel>
          <Input
            type="number"
            id="amount"
            disabled={isSubmitting}
            {...register("amount")}
          ></Input>
          <FormHelperText>amount of `{denom}` to be burned</FormHelperText>
        </FormControl>
        <FormControl isRequired my="5">
          <FormLabel>from address</FormLabel>
          <Input
            type="text"
            id="fromAddress"
            disabled={isSubmitting}
            {...register("fromAddress")}
          />
          <FormHelperText> address to be burned from</FormHelperText>
        </FormControl>
        <Button
          mt={4}
          colorScheme="teal"
          isLoading={isSubmitting}
          type="submit"
        >
          Burn
        </Button>
      </form>
    </Box>
  );
};

export default Burning;
