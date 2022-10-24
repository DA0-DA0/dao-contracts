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
import { useBurnAllowances, useDenom } from "../api/tokenfactoryIssuer";
import { AddressField, NumberField, ProposalMsgForm } from "./formHelpers";

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

// form
export const SetBurnerForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  return (
    <ProposalMsgForm
      msgType={"set_burner"}
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

export const BurnForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  return (
    <ProposalMsgForm
      msgType={"burn"}
      fields={[
        {
          name: "amount",
          isRequired: true,
          component: NumberField,
        },
        {
          name: "from_address",
          isRequired: true,
          component: AddressField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};
export default Burning;
