import { ActiveThreshold } from "./shared-types";

export type ExecuteMsg = {
update_active_threshold: {
new_threshold?: (ActiveThreshold | null)
[k: string]: unknown
}
}
