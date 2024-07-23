#[allow(clippy::collapsible_if)]
fn main() {}

mod dao;
pub use dao::*;

#[cfg(test)]
mod deploy;
#[cfg(test)]
mod tests;
