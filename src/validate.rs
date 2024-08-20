use cosmwasm_std::{Addr, Api, StdError, StdResult};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ValidateError {
  #[error("Not a valid {kind:?}: {reason:?}")]
  NotValid { kind: String, reason: String },
}

/// Data validation trait.
pub trait Validator<'a, T, U, E> {
  /// Validate a value.
  fn validate(&self, val: T) -> Result<U, E>;
}

impl<'a> Validator<'a, &'a str, Addr, StdError> for &'a dyn Api {
  /// Validate string into address. Calls [`Api:::addr_validate`], implemented for API
  /// consistency and demonstration.
  fn validate(&self, val: &'a str) -> StdResult<Addr> {
    self.addr_validate(val)
  }
}

impl<'a> Validator<'a, &'a str, Addr, ValidateError> for &'a dyn Api {
  /// Validate string into address. Calls [`Api:::addr_validate`] and wraps error in
  /// [`Error::Std`].
  fn validate(&self, val: &'a str) -> Result<Addr, ValidateError> {
    self
      .addr_validate(val)
      .map_err(|err| ValidateError::NotValid {
        kind: "Address".to_string(),
        reason: err.to_string(),
      })
  }
}
