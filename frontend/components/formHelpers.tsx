import { AddIcon } from "@chakra-ui/icons";
import {
  Alert,
  AlertIcon,
  Box,
  Button,
  Divider,
  FormControl,
  FormLabel,
  Heading,
  Input,
  Select,
  useToast,
} from "@chakra-ui/react";
import { fromBech32 } from "cosmwasm";
import { ExecuteMsg } from "cw-tokenfactory-issuer-sdk/types/contracts/TokenfactoryIssuer.types";
import {
  FieldErrors,
  FieldPath,
  FieldValues,
  useForm,
  UseFormRegister,
} from "react-hook-form";
import { getPrefix } from "../lib/conf";

export type FieldDef<Values extends FieldValues> = {
  name: FieldPath<Values>;
  isRequired: boolean;
  component: React.FC<FieldProps<Values>>;
};

export type AllKeys<T> = T extends any ? keyof T : never;
export type PickType<T, K extends AllKeys<T>> = T extends { [k in K]?: any }
  ? T[K]
  : never;

export function assertMsgType<M extends AllKeys<ExecuteMsg>>(m: M) {
  return m;
}

export function ProposalMsgForm<
  MessageType extends AllKeys<ExecuteMsg>,
  Values extends FieldValues
>({
  msgType,
  fields,
  onSubmitForm,
}: {
  msgType: MessageType;
  fields: FieldDef<Values>[];
  onSubmitForm: (msg: ExecuteMsg) => void;
}) {
  const toast = useToast();
  const {
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<Values>();
  const onSubmit = async (values: Values) => {
    // @ts-ignore
    onSubmitForm({
      [msgType]: values,
    });
    reset();
    toast({
      title: `'${msgType}' successfully added.`,
      status: "success",
      isClosable: true,
    });
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
        {msgType}
      </Heading>
      <Divider />
      <form onSubmit={handleSubmit(onSubmit)}>
        {fields.map(({ name, isRequired, component: Field }, i) => (
          <Field
            key={i}
            fieldName={name}
            register={register}
            errors={errors}
            isSubmitting={isSubmitting}
            isRequired={isRequired}
          />
        ))}
        <AddToProposalButton isSubmitting={isSubmitting} />
      </form>
    </Box>
  );
}

export function AddToProposalButton({
  isSubmitting,
}: {
  isSubmitting: boolean;
}) {
  return (
    <Button variant="outline" type="submit" isLoading={isSubmitting}>
      <AddIcon w={3} h={3} mr={3} /> Add to proposal
    </Button>
  );
}

export type FieldProps<Values extends FieldValues> = {
  fieldName: FieldPath<Values>;
  register: UseFormRegister<Values>;
  errors: FieldErrors<Values>;
  isSubmitting: boolean;
  isRequired: boolean;
};

export function NumberField<Values extends FieldValues>({
  fieldName,
  register,
  errors,
  isSubmitting,
  isRequired,
}: FieldProps<Values>) {
  const fieldNameString = String(fieldName);
  return (
    <FormControl isRequired={isRequired} my="5">
      <FormLabel>{fieldNameString}</FormLabel>
      <Input
        type="number"
        id={fieldNameString}
        disabled={isSubmitting}
        {...register(fieldName, {
          required: isRequired && `"${fieldNameString}" is required`,
        })}
      />

      {/* @ts-ignore */}
      <ValidateError message={errors[fieldName]?.message} />
    </FormControl>
  );
}

export function BooleanSelectField<Values extends FieldValues>({
  fieldName,
  register,
  errors,
  isSubmitting,
  isRequired,
}: FieldProps<Values>) {
  const fieldNameString = String(fieldName);
  return (
    <FormControl isRequired={isRequired} my="5">
      <FormLabel>{fieldNameString}</FormLabel>
      <Select
        id={fieldNameString}
        disabled={isSubmitting}
        {...register(fieldName, {
          setValueAs: (v) => {
            switch (v) {
              case "true":
                return true;
              case "false":
                return false;
              default:
                return undefined;
            }
          },
        })}
      >
        <option></option>
        <option value="true">true</option>
        <option value="false">false</option>
      </Select>

      {/* @ts-ignore */}
      <ValidateError message={errors[fieldName]?.message} />
    </FormControl>
  );
}

export function AddressField<Values extends FieldValues>({
  fieldName,
  register,
  errors,
  isSubmitting,
  isRequired,
}: FieldProps<Values>) {
  const fieldNameString = String(fieldName);
  return (
    <FormControl isRequired={isRequired} my="5">
      <FormLabel>{fieldNameString}</FormLabel>
      <Input
        type="text"
        id={fieldNameString}
        disabled={isSubmitting}
        {...register(fieldName, {
          required: isRequired && `"${fieldNameString}" is required`,
          validate: (address) => {
            try {
              const account = fromBech32(address);
              if (account.prefix !== getPrefix()) {
                return `Invalid address: prefix must be ${getPrefix()}`;
              }
            } catch (e) {
              return `Invalid address: ${e}`;
            }
          },
        })}
      />
      {/* @ts-ignore */}
      <ValidateError message={errors[fieldName]?.message} />
    </FormControl>
  );
}

export function ValidateError({ message }: { message: string | undefined }) {
  if (message) {
    return (
      <Alert status="error" variant="left-accent" mt="3">
        <AlertIcon />
        {message}
      </Alert>
    );
  }
  return <></>;
}
