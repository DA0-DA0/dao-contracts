import {
  Box,
  Heading,
  Table,
  TableContainer,
  Tbody,
  Td,
  Th,
  Tr,
} from "@chakra-ui/react";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import { useDenom, useMintAllowances } from "../api/tokenfactoryIssuer";
import {
  AddressField,
  assertMsgType,
  NumberField,
  PickType,
  ProposalMsgForm,
} from "./formHelpers";

const Minting = () => {
  const { data: denomRes } = useDenom();
  return (
    <Box>
      <Allowances></Allowances>
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

// form
export const SetMinterForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<N extends PickType<ExecuteMsg, "set_minter">>(
    name: keyof N
  ) {
    return name;
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("set_minter")}
      fields={[
        {
          name: assertName("address"),
          isRequired: true,
          component: AddressField,
        },
        {
          name: assertName("allowance"),
          isRequired: true,
          component: NumberField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};

export const MintForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<N extends PickType<ExecuteMsg, "mint">>(name: keyof N) {
    return name;
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("mint")}
      fields={[
        {
          name: assertName("to_address"),
          isRequired: true,
          component: AddressField,
        },
        {
          name: assertName("amount"),
          isRequired: true,
          component: NumberField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};

export default Minting;
