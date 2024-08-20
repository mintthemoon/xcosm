use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::CosmixResult;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum AuthError {
  #[error("Requestor is not authorized")]
  Unauthorized {},
}

/// Auth handler.
#[cw_serde]
pub enum Authorized<T: Eq+ToString=Addr> {
  /// Single authorized address.
  One(T),
  /// Multiple authorized addresses.
  Many(Vec<T>),
  /// No authorized addresses.
  None,
  /// All addresses authorized.
  Any,
}

impl<T: Eq+ToString> Authorized<T> {
  /// Create a new `Authorized` group.
  pub fn new<'a, U: Into<&'a [T]>>(group: U) -> Self
  where T: Clone+'a {
    let authorized: &[T] = group.into();
    match authorized.len() {
      0 => Authorized::None,
      1 => Authorized::One(authorized[0].clone()),
      _ => Authorized::Many(authorized.to_vec()),
    }
  }

  /// Authorize a single requestor.
  ///
  /// Requires requestor to match authorized.
  pub fn authorize(&self, requestor: &T) -> CosmixResult {
    match self {
      Authorized::One(authorized) => {
        if authorized != requestor {
          return Err(AuthError::Unauthorized {}.into());
        }
      }
      Authorized::Many(authorized) => {
        if !authorized.contains(requestor) {
          return Err(AuthError::Unauthorized {}.into());
        }
      }
      Authorized::None => return Err(AuthError::Unauthorized {}.into()),
      Authorized::Any => return Ok(()),
    };
    Ok(())
  }

  /// Authorize any of the requestors.
  ///
  /// Requires at least one of `requestors` to match authorized.
  pub fn authorize_any(&self, requestors: &Vec<T>) -> CosmixResult {
    match match self {
      Authorized::One(authorized) => requestors.contains(authorized),
      Authorized::Many(authorized) => requestors.iter().any(|r| authorized.contains(r)),
      Authorized::None => false,
      Authorized::Any => true,
    } {
      true => Ok(()),
      false => Err(AuthError::Unauthorized {}.into()),
    }
  }

  /// Authorize all of the requestors.
  ///
  /// Requires all of `requestors` to match authorized.
  pub fn authorize_all(&self, requestors: &Vec<T>) -> CosmixResult {
    match match self {
      Authorized::One(authorized) => requestors.contains(authorized),
      Authorized::Many(authorized) => requestors.iter().all(|r| authorized.contains(r)),
      Authorized::None => false,
      Authorized::Any => true,
    } {
      true => Ok(()),
      false => Err(AuthError::Unauthorized {}.into()),
    }
  }

  /// Authorize at least `min` of the requestors.
  ///
  /// Requires at least `min` of `requestors` to match authorized.
  pub fn authorize_at_least(&self, requestors: &Vec<T>, min: u32) -> CosmixResult {
    match match self {
      Authorized::One(authorized) => requestors.contains(authorized),
      Authorized::Many(authorized) => {
        requestors.iter().filter(|r| authorized.contains(r)).count() as u32 >= min
      }
      Authorized::None => false,
      Authorized::Any => true,
    } {
      true => Ok(()),
      false => Err(AuthError::Unauthorized {}.into()),
    }
  }
}

impl<T: Eq+ToString> Default for Authorized<T> {
  fn default() -> Self {
    Authorized::None
  }
}

impl<T: Eq+ToString, U: From<T>> Into<Vec<U>> for Authorized<T> {
  fn into(self) -> Vec<U> {
    match self {
      Authorized::One(authorized) => vec![authorized.into()],
      Authorized::Many(authorized) => authorized.into_iter().map(Into::into).collect(),
      Authorized::None => vec![],
      Authorized::Any => vec![],
    }
  }
}
