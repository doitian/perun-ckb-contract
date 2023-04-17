use ckb_occupied_capacity::{Capacity, IntoCapacity};
use ckb_testtool::ckb_types::core::{TransactionBuilder, TransactionView};
use ckb_testtool::ckb_types::packed::{TransactionViewBuilder, WitnessArgsBuilder};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        packed::{Byte32, CellInput, CellOutput, OutPoint, Script},
        prelude::Pack,
    },
    context::Context,
};
use ckb_types::prelude::*;
use perun_common::perun_types::{self, ChannelWitnessUnion};
use perun_common::redeemer;

use crate::perun::{self, harness};

use super::{ChannelId, FundingAgreement};

/// Build witness args containing the given action.
macro_rules! channel_witness {
    ($action:expr) => {
        WitnessArgsBuilder::default()
            .input_type(Some($action.as_bytes()).pack())
            .build()
    };
}

#[derive(Clone)]
pub struct OpenArgs {
    pub cid: ChannelId,
    pub funding_agreement: FundingAgreement,
    pub channel_token_outpoint: OutPoint,
    pub my_funds_outpoint: OutPoint,
    pub my_available_funds: Capacity,
    pub party_index: u8,
    pub pcls_code_hash: Byte32,
    pub pcls_script: Script,
    pub pcts_code_hash: Byte32,
    pub pcts_script: Script,
    pub pfls_code_hash: Byte32,
    pub pfls_script: Script,
}

pub struct OpenResult {
    pub tx: TransactionView,
    pub channel_cell: OutPoint,
    pub funds_cells: Vec<OutPoint>,
}

impl Default for OpenResult {
    fn default() -> Self {
        OpenResult {
            tx: TransactionBuilder::default().build(),
            channel_cell: OutPoint::default(),
            funds_cells: Vec::new(),
        }
    }
}

pub fn mk_open(
    ctx: &mut Context,
    env: &harness::Env,
    args: OpenArgs,
) -> Result<OpenResult, perun::Error> {
    let inputs = vec![
        CellInput::new_builder()
            .previous_output(args.channel_token_outpoint)
            .build(),
        CellInput::new_builder()
            .previous_output(args.my_funds_outpoint)
            .build(),
    ];
    let initial_cs =
        env.build_initial_channel_state(args.cid, args.party_index, &args.funding_agreement)?;
    let capacity_for_cs = env.min_capacity_for_channel(initial_cs.clone())?;
    let channel_cell = CellOutput::new_builder()
        .capacity(capacity_for_cs.pack())
        .lock(args.pcls_script.clone())
        .type_(Some(args.pcts_script.clone()).pack())
        .build();
    let wanted = args
        .funding_agreement
        .expected_funding_for(args.party_index)?;
    // TODO: Make sure enough funds available all cells!
    let fund_cell = CellOutput::new_builder()
        .capacity(wanted.into_capacity().pack())
        .lock(args.pfls_script)
        .build();
    let exchange_cell = create_funding_from(args.my_available_funds, wanted.into_capacity())?;
    // NOTE: The ORDER here is important. We need to reference the outpoints later on by using the
    // correct index in the output array of the transaction we build.
    let outputs = vec![
        (channel_cell.clone(), initial_cs.as_bytes()),
        (fund_cell.clone(), Bytes::new()),
        // Exchange cell.
        (
            CellOutput::new_builder()
                .capacity(exchange_cell.pack())
                .lock(env.always_success_script.clone())
                .build(),
            Bytes::new(),
        ),
    ];
    let outputs_data: Vec<_> = outputs.iter().map(|o| o.1.clone()).collect();
    let cell_deps = vec![
        env.always_success_script_dep.clone(),
        env.pcts_script_dep.clone(),
    ];
    let rtx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs.iter().map(|o| o.0.clone()))
        .outputs_data(outputs_data.pack())
        .cell_deps(cell_deps)
        .build();
    let tx = ctx.complete_tx(rtx);
    Ok(OpenResult {
        // See NOTE above for magic indices.
        channel_cell: OutPoint::new(tx.hash(), 0),
        funds_cells: vec![OutPoint::new(tx.hash(), 1)],
        tx,
    })
}

fn create_funding_from(
    available_capacity: Capacity,
    wanted_capacity: Capacity,
) -> Result<Capacity, perun::Error> {
    Ok(available_capacity.safe_sub(wanted_capacity)?)
}

pub struct AbortResult {
    pub tx: TransactionView,
}

pub fn mk_abort(
    ctx: &mut Context,
    env: &harness::Env,
    channel_cell: OutPoint,
    funds: Vec<OutPoint>,
) -> Result<AbortResult, perun::Error> {
    let abort_action = redeemer!(Abort);
    let witness_args = channel_witness!(abort_action);
    let mut inputs = vec![CellInput::new_builder()
        .previous_output(channel_cell)
        .build()];
    inputs.extend(
        funds
            .iter()
            .cloned()
            .map(|op| CellInput::new_builder().previous_output(op).build()),
    );

    let outputs = vec![CellOutput::new_builder()
        .capacity((1u64).into_capacity().pack())
        .lock(env.always_success_script.clone())
        .build()];
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .witness(witness_args.as_bytes().pack())
        .build();
    Ok(AbortResult {
        tx: ctx.complete_tx(tx),
    })
}
