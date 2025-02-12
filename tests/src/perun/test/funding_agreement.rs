use ckb_occupied_capacity::Capacity;
use ckb_testtool::ckb_types::packed::{Byte as PackedByte, Byte32, BytesBuilder};
use ckb_testtool::ckb_types::prelude::*;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::PublicKey;
use perun_common::perun_types::{
    self, Balances, BalancesBuilder, ParticipantBuilder, SEC1EncodedPubKeyBuilder,
};

use crate::perun;

#[derive(Clone)]
pub struct FundingAgreement(Vec<FundingAgreementEntry>);

impl FundingAgreement {
    pub fn new_with_capacities<P: perun::Account>(caps: Vec<(P, u64)>) -> Self {
        FundingAgreement(
            caps.iter()
                .enumerate()
                .map(|(i, (acc, c))| FundingAgreementEntry {
                    amounts: vec![(Asset::default(), *c)],
                    index: i as u8,
                    pub_key: acc.public_key(),
                })
                .collect(),
        )
    }

    pub fn content(&self) -> &Vec<FundingAgreementEntry> {
        &self.0
    }

    pub fn mk_participants(
        &self,
        payment_lock_hash: Byte32,
        payment_min_capacity: Capacity,
    ) -> Vec<perun_types::Participant> {
        self.0
            .iter()
            .map(|entry| {
                let sec1_encoded_bytes: Vec<_> = entry
                    .pub_key
                    .to_encoded_point(false)
                    .as_bytes()
                    .iter()
                    .map(|b| PackedByte::new(*b))
                    .collect();
                let sec1_pub_key = SEC1EncodedPubKeyBuilder::default()
                    .set(sec1_encoded_bytes.try_into().unwrap())
                    .build();
                let payment_args = BytesBuilder::default()
                    .set(vec![entry.index.into()])
                    .build();
                ParticipantBuilder::default()
                    .payment_lock_hash(payment_lock_hash.clone())
                    .payment_min_capacity(payment_min_capacity.pack())
                    // The tests use always success scripts which do not receive any args.
                    .unlock_args(Default::default())
                    // NOTE: The tests will pay out to an "address" encoded by the index of each
                    // participant for simplicity. This is relevant for the test-suite, because
                    // Perun channels should only be opened between participants that pay out to
                    // distinct addresses (i.e. distinct cells with distinct lockscripts).
                    .payment_args(payment_args)
                    .pub_key(sec1_pub_key)
                    .build()
            })
            .collect()
    }

    /// mk_balances creates a Balances object from the funding agreement where the given indices
    /// already funded their part.
    pub fn mk_balances(&self, indices: Vec<u8>) -> Result<Balances, perun::Error> {
        let uint128_balances = self.0.iter().fold(Ok(vec![]), |acc, entry| {
            match acc {
                Ok(mut acc) => {
                    match indices.iter().find(|&&i| i == entry.index) {
                        Some(_) => {
                            // We found the index in the list of funded indices, we expect the required
                            // amount for assets to be funded.
                            if let Some((Asset(0), amount)) = entry.amounts.iter().next() {
                                let amount128: u128 = (*amount).into();
                                acc.push(amount128.pack());
                                return Ok(acc);
                            } else {
                                return Err(perun::Error::from("unknown asset"));
                            };
                        }
                        None => {
                            // We did not find the index in the list of funded indices, the client
                            // identified by this index did not fund, yet.
                            acc.push(0u128.pack());
                            Ok(acc)
                        }
                    }
                }
                e => e,
            }
        })?;
        let bals = match uint128_balances.try_into() {
            Ok(bals) => bals,
            Err(_) => return Err(perun::Error::from("could not convert balances")),
        };
        Ok(BalancesBuilder::default().set(bals).build())
    }

    pub fn expected_funding_for(&self, index: u8) -> Result<u64, perun::Error> {
        let entry = self
            .0
            .iter()
            .find(|entry| entry.index == index)
            .ok_or("unknown index")?;
        entry
            .amounts
            .iter()
            .find_map(|e| {
                if let (Asset(0), amount) = e {
                    Some(*amount)
                } else {
                    None
                }
            })
            .ok_or("unsupported asset".into())
    }
}

#[derive(Clone)]
pub struct FundingAgreementEntry {
    pub amounts: Vec<(Asset, u64)>,
    pub index: u8,
    pub pub_key: PublicKey,
}

#[derive(Copy, Clone)]
pub struct Asset(pub u32);

impl Asset {
    pub fn new() -> Self {
        Asset(0)
    }
}

impl Default for Asset {
    fn default() -> Self {
        Asset(0)
    }
}
