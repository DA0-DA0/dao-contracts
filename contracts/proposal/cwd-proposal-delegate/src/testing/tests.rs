use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use crate::contract::{execute_delegate, execute};


// Non-admin cannot delegate a message
// Admin can delegate a message
#[test]
fn test_unauthorized_delegation() { 
    let deps = mock_dependencies();
    let env = mock_env(); 
    let info = mock_info("", &[]);

}

// Only delegated can execute
// Admin cannot execute
// Non-admin non-delegated cannot execute
#[test]
fn test_execute_authorization() {
    
}


// Can only execute once for policy
// Can execute multiple if `preserve_on_failure` is true 
// and execution fails. 
#[test]
fn test_execute_on_failure_policy() {

}

// Cannot execute if expired
#[test]
fn test_execute_on_expired() {

}
