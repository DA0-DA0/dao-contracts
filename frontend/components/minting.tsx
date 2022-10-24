import {
  Box,
  Heading,
  Table,
  TableContainer,
  Tbody,
  Td,
  Th,
  Tr,
  VStack,
} from "@chakra-ui/react";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import { useDenom, useMintAllowances } from "../api/tokenfactoryIssuer";
import { AddressField, NumberField, ProposalMsgForm } from "./formHelpers";

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

// form
export const SetMinterForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  return (
    <ProposalMsgForm
      msgType={"set_minter"}
      fields={[
        {
          name: "allowance",
          isRequired: true,
          component: NumberField,
        },
        {
          name: "address",
          isRequired: true,
          component: AddressField,
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
  return (
    <ProposalMsgForm
      msgType={"mint"}
      fields={[
        {
          name: "amount",
          isRequired: true,
          component: NumberField,
        },
        {
          name: "to_address",
          isRequired: true,
          component: AddressField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};

export default Minting;
