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
import { useBurnAllowances, useDenom } from "../api/tokenfactoryIssuer";
import {
  AddressField,
  assertMsgType,
  NumberField,
  PickType,
  ProposalMsgForm,
} from "./formHelpers";

const Burning = () => {
  const { data: denomRes } = useDenom();
  return (
    <Box>
      <Allowances></Allowances>
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
  function assertName<N extends PickType<ExecuteMsg, "set_burner">>(
    name: keyof N
  ) {
    return name;
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("set_burner")}
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

export const BurnForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<N extends PickType<ExecuteMsg, "burn">>(name: keyof N) {
    return name;
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("burn")}
      fields={[
        {
          name: assertName("from_address"),
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
export default Burning;
