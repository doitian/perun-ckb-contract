// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc;

use perun_common::perun_types::ChannelParameters;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{
        load_cell_data, load_cell_lock_hash, load_script, load_transaction, load_tx_hash,
        load_witness_args,
    },
    syscalls::SysError,
};

use crate::error::Error;

pub fn main() -> Result<(), Error> {
    // remove below examples and write your code here

    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:?}", args);

    // return an error if args is invalid
    if args.is_empty() {
        return Err(Error::NoArgs);
    }

    // cell_deps -> point to reference cells for signature verification.
    //   => Load code from cell deps dynamically and access below.
    // let cp = ChannelParameters::from_slice(&args).unwrap();
    // We will access only the first witness, because our lockscript does not
    // require any more witnesses.
    let wits = load_witness_args(0, Source::GroupInput)?
        .lock()
        // BytesOpt could be empty, we will cast to Option<Bytes> and unwrap
        // manually.
        .to_opt()
        .ok_or(Error::NoWitness)?
        .unpack();

    verify_valid_participant(&wits, &args)?;

    let inputs_amount = collect_inputs_amount()?;
    let outputs_amount = collect_outputs_amount()?;

    if inputs_amount != outputs_amount {
        return Err(Error::Amount);
    }

    return Ok(());
}

pub fn verify_valid_participant(witness: &Bytes, args: &Bytes) -> Result<(), Error> {
    let tx_hash = load_tx_hash()?;
    Ok(())
}

pub fn check_owner_mode(args: &Bytes) -> Result<bool, Error> {
    for i in 0.. {
        // Loop over all input cells.
        let lock_hash = match load_cell_lock_hash(i, Source::Input) {
            Ok(lock_hash) => lock_hash,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(err) => return Err(err.into()),
        };
        // This checks if the lock_hash matches the args. Also Rust assures,
        // that both arrays must be of the same length, because lock_hash is
        // always 32 bytes long.
        if args[..] == lock_hash[..] {
            let y = args[31];
            return Ok(true);
        }
    }
    Ok(false)
}

const UDT_LEN: usize = 16;

fn collect_inputs_amount() -> Result<u128, Error> {
    collect_amount_for_source(Source::GroupInput)
}

fn collect_outputs_amount() -> Result<u128, Error> {
    collect_amount_for_source(Source::GroupOutput)
}

fn collect_amount_for_source(s: Source) -> Result<u128, Error> {
    let mut amount: u128 = 0;
    let mut buf = [0u8; UDT_LEN];

    for i in 0.. {
        let data = match load_cell_data(i, s) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => return Err(err.into()),
        };

        if data.len() != UDT_LEN {
            return Err(Error::Encoding);
        }
        buf.copy_from_slice(&data);
        amount += u128::from_le_bytes(buf);
    }

    Ok(amount)
}
