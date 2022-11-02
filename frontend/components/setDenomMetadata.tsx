import {
  FormControl,
  FormHelperText,
  FormLabel,
  Input,
  Skeleton,
  Textarea,
} from "@chakra-ui/react";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import { FieldValues } from "react-hook-form";
import { useDenom } from "../api/tokenfactoryIssuer";
import {
  assertMsgType,
  FieldProps,
  PickType,
  ProposalMsgForm,
  TextField,
  ValidateError,
} from "./formHelpers";

export const SetDenomMetadataForm = ({
  onSubmitForm,
}: {
  onSubmitForm: (msg: ExecuteMsg) => void;
}) => {
  function assertName<
    N extends PickType<ExecuteMsg, "set_denom_metadata">["metadata"]
  >(name: keyof N) {
    return String(name);
  }
  return (
    <ProposalMsgForm
      msgType={assertMsgType("set_denom_metadata")}
      fields={[
        {
          name: assertName("base"),
          isRequired: true,
          component: BaseField,
        },
        {
          name: assertName("denom_units"),
          isRequired: true,
          component: DenomUnitField,
        },
        {
          name: assertName("description"),
          isRequired: true,
          component: TextField,
        },
        {
          name: assertName("display"),
          isRequired: true,
          component: TextField,
          helperText:
            "display indicates the suggested denom that should be displayed in clients.",
        },
        {
          name: assertName("name"),
          isRequired: true,
          component: TextField,
          helperText: "name defines the name of the token (eg: Cosmos Atom)",
        },
        {
          name: assertName("symbol"),
          isRequired: true,
          component: TextField,
          helperText:
            "symbol is the token symbol usually shown on exchanges (eg: ATOM). This can be the same as the display.",
        },
      ]}
      beforeOnSubmit={(v) => {
        const lines = v["denom_units"].split("\n");
        const denom_units = lines.map((d: string) => extractDenomUnits(d));
        return { metadata: { ...v, denom_units } };
      }}
      onSubmitForm={onSubmitForm}
    />
  );
};

export function BaseField<Values extends FieldValues>({
  fieldName,
  register,
  errors,
  isRequired,
  helperText,
}: FieldProps<Values>) {
  const fieldNameString = String(fieldName);
  const { data, error } = useDenom();

  return (
    <FormControl isRequired={isRequired} my="5">
      <FormLabel>{fieldNameString}</FormLabel>
      <Skeleton isLoaded={typeof data !== "undefined"}>
        <Input
          type="text"
          id={fieldNameString}
          value={data?.denom || ""}
          {...register(fieldName, {
            required: isRequired && `"${fieldNameString}" is required`,
          })}
        />
      </Skeleton>

      {/* @ts-ignore */}
      <ValidateError message={error || errors[fieldName]?.message} />
      {typeof helperText !== "undefined" && (
        <FormHelperText>{helperText}</FormHelperText>
      )}
    </FormControl>
  );
}

export const extractDenomUnits = (line: string) => {
  const [denom, exponent, aliasesStr] = line
    ?.trim()
    .split("|")
    ?.map((v) => v.trim());
  const aliases = aliasesStr?.split(",")?.map((v) => v.trim()) || [];
  return { denom, exponent, aliases };
};

export function DenomUnitField<Values extends FieldValues>({
  fieldName,
  register,
  errors,
  isRequired,
  helperText,
}: FieldProps<Values>) {
  const fieldNameString = String(fieldName);
  const { data, error } = useDenom();

  return (
    <FormControl isRequired={isRequired} my="5">
      <FormLabel>{fieldNameString}</FormLabel>

      <Skeleton isLoaded={typeof data !== "undefined"}>
        <Textarea
          id={fieldNameString}
          {...register(fieldName, {
            required: isRequired && `"${fieldNameString}" is required`,
            validate: (value: string) => {
              const lines = value.split("\n");
              for (let i in lines) {
                const { denom, exponent } = extractDenomUnits(lines[i] || "");

                // === denom ===
                if (!denom) {
                  return `"denom" is required (line ${i})`;
                }
                // denom 0 must be exactly factory/<address>/<subdenom>
                if (i === "0" && denom !== data?.denom) {
                  return `"denom" on line 0 must be exactly "${data?.denom}"`;
                }

                // === exponent ===
                if (!exponent) {
                  return `"exponent" is required (line ${i})`;
                }

                if (exponent.match("^\\d+$") === null) {
                  return `"exponent" must be number (line ${i})`;
                }
                // exponent 0 must be exactly 0
                if (i === "0" && exponent !== "0") {
                  return `"exponent" on line 0 must be exactly "0"`;
                }
              }
            },
          })}
        />
      </Skeleton>

      {/* @ts-ignore */}
      <ValidateError message={error || errors[fieldName]?.message} />
      {typeof helperText !== "undefined" && (
        <FormHelperText>{helperText}</FormHelperText>
      )}
    </FormControl>
  );
}
