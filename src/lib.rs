pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod routes;
pub mod startup;
pub mod telemetry;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;
