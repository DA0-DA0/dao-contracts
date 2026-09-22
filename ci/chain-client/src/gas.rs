use std::{collections::HashMap, panic::Location};

use serde::{Deserialize, Serialize};

use crate::rpc::Receipt;

pub type GasReport = HashMap<String, HashMap<String, GasEntry>>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GasEntry {
    pub gas_wanted: u64,
    pub gas_used: u64,
    pub file_name: String,
    pub line_number: u32,
}

pub(crate) fn record(
    report: &mut Option<GasReport>,
    contract: &str,
    operation: &str,
    kind: &str,
    receipt: &Receipt,
    caller: &Location<'_>,
) {
    if let Some(report) = report {
        report.entry(contract.into()).or_default().insert(
            format!("{kind}__{operation}"),
            GasEntry {
                gas_wanted: receipt.gas_wanted,
                gas_used: receipt.gas_used,
                file_name: caller.file().into(),
                line_number: caller.line(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn report_shape_and_operation_keys_remain_compatible() {
        let mut report = Some(GasReport::new());
        let receipt = Receipt {
            hash: "hash".into(),
            height: 1,
            code: 0,
            codespace: String::new(),
            log: String::new(),
            data: vec![],
            events: vec![],
            gas_wanted: 120,
            gas_used: 100,
        };
        let caller = Location::caller();
        record(&mut report, "contract", "Store", "Store", &receipt, caller);
        let encoded = serde_json::to_value(report.unwrap()).unwrap();
        assert_eq!(
            encoded,
            json!({"contract":{"Store__Store":{
                "gas_wanted":120,"gas_used":100,"file_name":caller.file(),"line_number":caller.line()
            }}})
        );
        let mut disabled = None;
        record(
            &mut disabled,
            "contract",
            "execute",
            "Execute",
            &receipt,
            caller,
        );
        assert!(disabled.is_none());
    }
}
