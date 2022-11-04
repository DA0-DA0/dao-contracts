import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import { MultipleAddressField, ProposalMsgForm } from "./formHelpers";

// form
export const UpdateMembers = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  return (
    <ProposalMsgForm
      msgType={"update_members"}
      fields={[
        {
          name: "remove",
          isRequired: false,
          component: MultipleAddressField,
        },
        {
          name: "add",
          isRequired: false,
          component: MultipleAddressField,
        },
      ]}
      onSubmitForm={onSubmitForm}
      beforeOnSubmit={(v) => {
        // remove has type Vec<cw4::Member>
        const remove = v["remove"]
          .split(",")
          .map((addr: string) => addr.trim())
          .filter((addr: string) => addr !== "");
        const add = v["add"]
          .split(",")
          .map((addr: string) => addr.trim())
          .filter((addr: string) => addr !== "")
          .map((addr: string) => ({
            addr,
            weight: 1,
          }));

        return { add, remove };
      }}
    />
  );
};
