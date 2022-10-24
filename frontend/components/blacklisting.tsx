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
import {
  useBlacklistees,
  useBlacklisterAllowances,
  useDenom,
} from "../api/tokenfactoryIssuer";
import {
  AddressField,
  BooleanSelectField,
  ProposalMsgForm,
} from "./formHelpers";

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
export const SetBlacklisterForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  return (
    <ProposalMsgForm
      msgType={"set_blacklister"}
      fields={[
        {
          name: "address",
          isRequired: true,
          component: AddressField,
        },
        {
          name: "status",
          isRequired: true,
          component: BooleanSelectField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};

export const BlacklistForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  return (
    <ProposalMsgForm
      msgType={"blacklist"}
      fields={[
        {
          name: "address",
          isRequired: true,
          component: AddressField,
        },
        {
          name: "status",
          isRequired: true,
          component: BooleanSelectField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};

export default Blacklistng;
