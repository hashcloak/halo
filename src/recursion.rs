use super::{Curve, Field, Subsonic, SubsonicProof, ProofMetadata, ProvingSystem};
use super::circuits::*;
use super::gadgets::*;
use super::synthesis::Basic;
use std::marker::PhantomData;

pub struct RecursiveProof<E1: Curve, E2: Curve> {
    proof: SubsonicProof<E1>,
    oldproof1: ProofMetadata<E1>,
    oldproof2: ProofMetadata<E2>,
    deferred: Deferred<E1>,

    is_valid: bool,

    _marker: PhantomData<(E1, E2)>,
}

#[derive(Clone)]
pub struct RecursiveParameters<E1: Curve, E2: Curve> {
    a: Subsonic<E1>,
    b: Subsonic<E2>,
}

impl<E1: Curve, E2: Curve> RecursiveParameters<E1, E2> {
    pub fn new(k: usize) -> Self {
        RecursiveParameters {
            a: Subsonic::new(k),
            b: Subsonic::new(k),
        }
    }

    pub fn switch(self) -> RecursiveParameters<E2, E1> {
        RecursiveParameters {
            a: self.b,
            b: self.a,
        }
    }
}

struct VerificationCircuit<'a, C1: Curve, C2: Curve, CS: Circuit<C1::Scalar>> {
    _marker: PhantomData<(C1, C2)>,
    inner_circuit: &'a CS,
    proof: &'a SubsonicProof<C2>,
    old_payload: &'a [u8],
    new_payload: &'a [u8],
}

impl<'a, E1: Curve, E2: Curve<Base=E1::Scalar>, Inner: Circuit<E1::Scalar>> Circuit<E1::Scalar>
    for VerificationCircuit<'a, E1, E2, Inner>
{
    fn synthesize<CS: ConstraintSystem<E1::Scalar>>(
        &self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        // Witness the commitment to r(X, Y)
        let r_commitment = CurvePoint::<E2>::alloc(cs, || {
            let (x, y) = self.proof.r_commitment.get_xy().unwrap();
            Ok((x, y))
        })?;

        // The transcript starts out with value zero.
        let transcript = AllocatedNum::alloc(cs, || {
            Ok(E1::Scalar::zero())
        })?;
        cs.enforce_zero(transcript.lc());

        // Hash the commitment to r(X, Y)
        let transcript = append_point(cs, &transcript, &r_commitment)?;

        // Obtain the challenge y_cur
        let (transcript, y_cur) = obtain_challenge(cs, &transcript)?;

        self.inner_circuit.synthesize(cs)
    }
}

impl<E1: Curve, E2: Curve<Base=E1::Scalar>> RecursiveProof<E1, E2> {
    pub fn create_proof<CS: Circuit<E1::Scalar> + Circuit<E2::Scalar>>(
        params: &RecursiveParameters<E1, E2>,
        old_proof: &RecursiveProof<E2, E1>,
        circuit: &CS,
        old_payload: &[u8],
        new_payload: &[u8],
    ) -> Result<Self, SynthesisError> {
        let circuit = VerificationCircuit::<E1, E2, _> {
            _marker: PhantomData,
            inner_circuit: circuit,
            proof: &old_proof.proof,
            old_payload,
            new_payload,
        };

        let proof = params
            .a
            .new_proof::<_, Basic>(&circuit, &old_proof.oldproof2)?;

        let proof_metadata = params.a.get_proof_metadata::<_, Basic>(&circuit, &proof)?;

        // arithmetic we need done by the parent proof system which can
        // do it more efficiently
        let deferred = Deferred {
            _marker: PhantomData,
        };

        Ok(RecursiveProof {
            proof,
            oldproof1: proof_metadata,
            oldproof2: old_proof.oldproof1.clone(),
            deferred,

            is_valid: true,

            _marker: PhantomData,
        })
    }

    pub fn create_false_proof() -> Self {
        RecursiveProof {
            proof: SubsonicProof::dummy(),
            oldproof1: ProofMetadata::dummy(),
            oldproof2: ProofMetadata::dummy(),
            deferred: Deferred::dummy(),

            is_valid: false,

            _marker: PhantomData,
        }
    }

    pub fn verify_proof<CS: Circuit<E1::Scalar>>(
        &self,
        params: &RecursiveParameters<E1, E2>,
        circuit: &CS,
        payload: &[u8],
    ) -> Result<bool, SynthesisError> {
        params.a.verify_proof::<_, Basic>(circuit, &self.proof, &[])
    }
}

#[derive(Clone)]
struct Deferred<C: Curve> {
    _marker: PhantomData<C>, /*
                             g: C,
                             challenges: Vec<C::Scalar>,
                             s_new: C,
                             y_new: C::Scalar
                             */
}

impl<C: Curve> Deferred<C> {
    fn dummy() -> Self {
        Deferred {
            _marker: PhantomData,
        }
    }
}