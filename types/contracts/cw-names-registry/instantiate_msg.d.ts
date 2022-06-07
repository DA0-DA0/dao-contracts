import { PaymentInfo } from "./shared-types";

export interface InstantiateMsg {
admin: string
payment_info: PaymentInfo
[k: string]: unknown
}
