use std::collections::{hash_map::Entry, HashMap};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, CosmosMsg};
use derive_deref::{Deref, DerefMut};

use crate::{
  math::{ContainerError, TryMinusMut, TryPlusMut, ValueError},
  validate::{ApiValidator, ValidateResult},
  CoinError, CoinSet, IntoResult, MathError, ValidateError, Validator,
};

pub type FundResult<T=()> = Result<T, FundError>;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum FundError {
  #[error("Coin error during fund operation: {0}")]
  Coin(#[from] CoinError),

  #[error("Math error during fund operation: {0}")]
  Math(#[from] MathError),

  #[error("Data validation error during fund operation: {0}")]
  Validate(#[from] ValidateError),

  #[error("Fund distribution claims cannot exceed 100%")]
  DistributionOverclaimed {},

  #[error("Fund distribution must contain at least one claim")]
  DistributionUnclaimed {},

  #[error("Unexpected fund error: {msg:?}")]
  Unexpected { msg: String },
}

#[cw_serde]
#[derive(Copy)]
pub struct Claim(u32);

impl Claim {
  pub fn bps(&self) -> u32 {
    self.0
  }

  pub fn claim(&self, funds: &CoinSet) -> FundResult<CoinSet> {
    let mut claimed = funds.clone();
    for (_, amount) in claimed.iter_mut() {
      *amount = self.claim_amount(amount.u128())?.into();
    }
    Ok(claimed)
  }

  pub fn claim_amount(&self, total: u128) -> FundResult<u128> {
    total
      .checked_mul(self.bps() as u128)
      .ok_or(MathError::Container(ContainerError::Overflow {}))?
      .checked_div(100000u128)
      .ok_or(MathError::Value(ValueError::DivideByZero {}))
      .into_result()
  }
}

#[cw_serde]
#[derive(Deref, DerefMut)]
pub struct Distribution(HashMap<Addr, Claim>);

impl Distribution {
  pub fn new(claims: HashMap<Addr, Claim>) -> Self {
    Distribution(claims)
  }

  pub fn claims(&self) -> &HashMap<Addr, Claim> {
    &self.0
  }

  pub fn total_bps(&self) -> FundResult<u32> {
    let total = self.claims().iter().map(|(_, claim)| claim.bps()).sum();
    if total > 10000 {
      return Err(FundError::DistributionOverclaimed {});
    }
    Ok(total)
  }

  pub fn with_remainder_to(&self, addr: Addr) -> FundResult<Self> {
    let rem_claim = Claim(10000 - self.total_bps()?);
    let mut claims = self.claims().clone();
    match claims.entry(addr) {
      Entry::Vacant(entry) => {
        entry.insert(rem_claim);
      }
      Entry::Occupied(mut entry) => {
        let claim = entry.get_mut();
        claim.0 += rem_claim.bps();
      }
    }
    Ok(Self(claims))
  }

  pub fn distribute_coins(&self, from: &Addr, funds: &CoinSet) -> FundResult<CosmosMsg> {
    if self.claims().len() == 0 {
      return Err(FundError::DistributionUnclaimed {});
    }
    let mut rem = funds.clone();
    let mut claimed = self
      .claims()
      .iter()
      .map(|(addr, claim)| {
        let claimed = claim.claim(funds)?;
        rem.try_minus_mut(&claimed)?;
        Ok((addr, claim.claim(funds)?))
      })
      .collect::<FundResult<Vec<(&Addr, CoinSet)>>>()?;
    // give remainder to first claim
    // TODO make this behavior configurable
    claimed
      .first_mut()
      .map(|(_, coins)| coins.try_plus_mut(&rem))
      .transpose()?
      .ok_or_else(|| FundError::Unexpected {
        msg: "distribution claims are not empty but no claimed funds were calculated".to_string(),
      })?;
    funds.send_many(from, claimed).into_result()
  }
}

impl Default for Distribution {
  fn default() -> Self {
    Self::new(HashMap::new())
  }
}

impl From<HashMap<Addr, Claim>> for Distribution {
  fn from(claims: HashMap<Addr, Claim>) -> Self {
    Self(claims)
  }
}

#[cw_serde]
#[derive(Deref, DerefMut)]
pub struct DistributionMsg(HashMap<String, Claim>);

impl<'a> ApiValidator<'a, Distribution> for &DistributionMsg {
  fn api_validate(self, api: &dyn Api) -> ValidateResult<Distribution> {
    self
      .iter()
      .map(|(addr_str, claim)| Ok::<_, ValidateError>((api.validate(&addr_str)?, *claim)))
      .collect::<ValidateResult<HashMap<Addr, Claim>>>()
      .map(Into::into)
  }
}
