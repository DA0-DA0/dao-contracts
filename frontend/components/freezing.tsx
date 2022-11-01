import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import {
  AddressField,
  assertMsgType,
  BooleanSelectField,
  PickType,
  ProposalMsgForm,
} from "./formHelpers";

export const SetFreezerForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<N extends PickType<ExecuteMsg, "set_freezer">>(
    name: keyof N
  ) {
    return name;
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("set_freezer")}
      fields={[
        {
          name: assertName("address"),
          isRequired: true,
          component: AddressField,
        },
        {
          name: assertName("status"),
          isRequired: true,
          component: BooleanSelectField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};

export const FreezeForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<N extends PickType<ExecuteMsg, "freeze">>(name: keyof N) {
    return name;
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("freeze")}
      fields={[
        {
          name: assertName("status"),
          isRequired: true,
          component: BooleanSelectField,
        },
      ]}
      onSubmitForm={onSubmitForm}
    />
  );
};
