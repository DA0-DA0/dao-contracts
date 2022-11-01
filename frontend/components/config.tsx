import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import {
  AddressField,
  assertMsgType,
  PickType,
  ProposalMsgForm,
} from "./formHelpers";

export const ChangeTokenFactoryAdminForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<
    N extends PickType<ExecuteMsg, "change_token_factory_admin">
  >(name: keyof N) {
    return name;
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("change_token_factory_admin")}
      fields={[
        {
          name: assertName("new_admin"),
          isRequired: true,
          component: AddressField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};

export const ChangeContractOwnerForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<N extends PickType<ExecuteMsg, "change_contract_owner">>(
    name: keyof N
  ) {
    return name;
  }

  return (
    <ProposalMsgForm
      msgType={assertMsgType("change_contract_owner")}
      fields={[
        {
          name: assertName("new_owner"),
          isRequired: true,
          component: AddressField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};
