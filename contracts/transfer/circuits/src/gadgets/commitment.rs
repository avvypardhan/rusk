// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    JubJubAffine, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use plonk_gadgets::AllocatedScalar;

/// Prove knowledge of the value and blinding factor, which make up the value commitment.
/// This commitment gadget is using the pedersen commitments.
/// C = a*g + b*h
pub fn commitment(
    composer: &mut StandardComposer,
    value: AllocatedScalar,
    blinder: AllocatedScalar,
    pub_commit: JubJubAffine,
) {
    let p1 = scalar_mul(composer, value.var, GENERATOR_EXTENDED);
    let p2 = scalar_mul(composer, blinder.var, GENERATOR_NUMS_EXTENDED);

    let commitment = p1.point().fast_add(composer, *p2.point());

    composer.assert_equal_public_point(commitment, pub_commit);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use anyhow::{Error, Result};
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};

    #[test]
    fn commitment_gadget() -> Result<(), Error> {
        let value = JubJubScalar::from(100 as u64);
        let blinder = JubJubScalar::from(20000 as u64);

        let pc_commitment = JubJubAffine::from(
            &(GENERATOR_EXTENDED * value)
                + &(GENERATOR_NUMS_EXTENDED * blinder),
        );

        // Generate Composer & Public Parameters
        let pub_params =
            PublicParameters::setup(1 << 14, &mut rand::thread_rng())?;
        let (ck, vk) = pub_params.trim(1 << 13)?;
        let mut prover = Prover::new(b"test");

        let value =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(100));
        let blinder =
            AllocatedScalar::allocate(prover.mut_cs(), BlsScalar::from(20000));

        commitment(prover.mut_cs(), value, blinder, pc_commitment);

        prover.preprocess(&ck)?;
        let proof = prover.prove(&ck)?;

        let mut verifier = Verifier::new(b"test");

        let value =
            AllocatedScalar::allocate(verifier.mut_cs(), BlsScalar::from(100));
        let blinder = AllocatedScalar::allocate(
            verifier.mut_cs(),
            BlsScalar::from(20000),
        );

        commitment(
            verifier.mut_cs(),
            value,
            blinder,
            JubJubAffine::from(pc_commitment),
        );
        verifier.preprocess(&ck)?;

        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi)
    }
}
