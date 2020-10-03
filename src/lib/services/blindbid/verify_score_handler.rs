// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::super::ServiceRequestHandler;
use super::get_bid_storage_fields;
use super::{VerifyScoreRequest, VerifyScoreResponse};
use crate::circuit_helpers::*;
use crate::encoding::decode_request_param;
use anyhow::Result;
use dusk_blindbid::score_gen::Score;
use dusk_blindbid::BlindBidCircuit;
use dusk_plonk::jubjub::AffinePoint as JubJubAffine;
use dusk_plonk::prelude::*;
use tonic::{Request, Response, Status};

/// Implementation of the VerifyScore Handler.
pub struct VerifyScoreHandler<'a> {
    request: &'a Request<VerifyScoreRequest>,
}

impl<'a, 'b>
    ServiceRequestHandler<'a, 'b, VerifyScoreRequest, VerifyScoreResponse>
    for VerifyScoreHandler<'a>
where
    'b: 'a,
{
    fn load_request(request: &'b Request<VerifyScoreRequest>) -> Self {
        Self { request }
    }

    fn handle_request(&self) -> Result<Response<VerifyScoreResponse>, Status> {
        // Get the optional parameters from the request.
        let (proof, score, seed, prover_id) =
            parse_score_verify_params(self.request)?;
        // Get the non-optional parameters from the request.
        let latest_consensus_round =
            BlsScalar::from(self.request.get_ref().round as u64);
        let latest_consensus_step =
            BlsScalar::from(self.request.get_ref().step as u64);
        // Get bid from storage
        let (bid, branch) = get_bid_storage_fields(
            self.request.get_ref().index_stored_bid as usize,
            // XXX: These values are hardcoded so that we can mock the
            // representation inside of the tree of the same Bid that should
            // be previously allocated.
            None,
            None,
        )?;

        // Create a BlindBidCircuit instance
        let mut circuit = BlindBidCircuit {
            bid: Some(bid),
            score: Some(Score::default()),
            secret_k: Some(BlsScalar::default()),
            secret: Some(JubJubAffine::default()),
            seed: Some(seed),
            latest_consensus_round: Some(latest_consensus_round),
            latest_consensus_step: Some(latest_consensus_step),
            branch: Some(&branch),
            size: 0,
            pi_constructor: None,
        };

        Ok(Response::new(VerifyScoreResponse {
            success: match verify_blindbid_proof(
                &mut circuit,
                &proof,
                prover_id,
                score,
            ) {
                Ok(_) => true,
                Err(_) => false,
            },
        }))
    }
}

// Parses the optional inputs of the VerifyScoreRequest returning an error if
// any of them isn't present (is `None`).
fn parse_score_verify_params(
    request: &Request<VerifyScoreRequest>,
) -> Result<(Proof, BlsScalar, BlsScalar, BlsScalar), Status> {
    let proof: Proof =
        decode_request_param(request.get_ref().proof.as_ref().as_ref())?;
    let score: BlsScalar =
        decode_request_param(request.get_ref().score.as_ref().as_ref())?;
    let seed: BlsScalar =
        decode_request_param(request.get_ref().seed.as_ref().as_ref())?;
    let prover_id: BlsScalar =
        decode_request_param(request.get_ref().prover_id.as_ref().as_ref())?;
    Ok((proof, score, seed, prover_id))
}

/// Given a circuit instance loaded with the dummy inputs and a
/// blindbid proof, verify whether the proof is correct or not.
fn verify_blindbid_proof(
    circuit: &mut BlindBidCircuit,
    proof: &Proof,
    prover_id: BlsScalar,
    score: BlsScalar,
) -> Result<()> {
    // Read VerifierKey of the circuit.
    let verifier_key = read_blindcid_circuit_vk()?;
    // Build PI array (safe to unwrap since we just created the circuit
    // with everything initialized).
    let pi = vec![
        PublicInput::BlsScalar(
            -circuit.branch.expect("Unexpected Error").root,
            0,
        ),
        PublicInput::BlsScalar(
            -circuit.bid.expect("Unexpected Error").hash(),
            0,
        ),
        PublicInput::AffinePoint(
            circuit.bid.expect("Unexpected Error").c,
            0,
            0,
        ),
        PublicInput::BlsScalar(
            -circuit.bid.expect("Unexpected Error").hashed_secret,
            0,
        ),
        PublicInput::BlsScalar(-prover_id, 0),
        PublicInput::BlsScalar(-score, 0),
    ];
    // Verify the proof.
    circuit.verify_proof(
        &crate::PUB_PARAMS,
        &verifier_key,
        b"BlindBid",
        proof,
        &pi,
    )
}