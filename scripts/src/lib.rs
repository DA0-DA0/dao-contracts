#[allow(clippy::collapsible_if)]
fn main() {}

mod dao;
mod external;
pub use dao::*;
pub use external::*;

#[cfg(test)]
mod tests;