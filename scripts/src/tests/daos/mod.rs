use cosmwasm_std::{Addr, Event};

pub mod voting;


/// grabs prop module from app response
pub fn extract_dao_events(events: &Vec<Event>, prop_type: &str) -> Option<Addr> {
    for event in events {
        if event.ty == "wasm" {
            for attribute in &event.attributes {
                if attribute.key == prop_type {
                    return Some(Addr::unchecked(&attribute.value));
                }
            }
        }
    }
    None
}
// grabs prop module from app response
// pub fn extract_prop_id(events: &Vec<Event>) -> Option<u64> {
//     for event in events {
//         if event.ty == "wasm" {
//             for attribute in &event.attributes {
//                 if attribute.key == "proposal_id" {
//                     return Some(u64::from_str_radix(&attribute.value, 10).unwrap());
//                 }
//             }
//         }
//     }
//     None
// }