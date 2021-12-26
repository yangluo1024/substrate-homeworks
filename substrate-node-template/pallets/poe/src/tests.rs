use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use super::*;

#[test]
fn create_claim_works() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2];
        assert_ok!(PoeModule::create_claim(Origin::signed(1),proof.clone()));
        assert_eq!(
            Proofs::<Test>::get(&proof),
            Some((1, <frame_system::Pallet<Test>>::block_number())),
        );
    });
}

#[test]
fn create_claim_failed_when_proof_too_long() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2, 3, 4, 5, 6, 7];
        assert_noop!(
            PoeModule::create_claim(Origin::signed(1), proof.clone()), 
            Error::<Test>::ProofTooLong,
        );
    });
}

#[test]
fn create_claim_failed_when_claim_already_exist() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2];
        let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
        assert_noop!(
            PoeModule::create_claim(Origin::signed(1), proof.clone()),
            Error::<Test>::ProofAlreadyClaimed,
        );
    });
}

#[test]
fn revoke_claim_works() {
    new_test_ext().execute_with(|| {
        let proof = vec![1,2];
        let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
        assert_ok!(PoeModule::revoke_claim(Origin::signed(1), proof.clone()));
        assert_eq!(Proofs::<Test>::get(&proof), None);
    });
}

#[test]
fn revoke_claim_failed_when_no_such_proof() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2];
        assert_noop!(
            PoeModule::revoke_claim(Origin::signed(1), proof),
            Error::<Test>::NoSuchProof,
        );
    });
}

#[test]
fn revoke_claim_failed_when_not_proof_owner() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2];
        let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
        assert_noop!(
            PoeModule::revoke_claim(Origin::signed(2), proof),
            Error::<Test>::NotProofOwner,
        );
    });
}

#[test]
fn transfer_claim_works() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2];
        let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
        assert_ok!(PoeModule::transfer_claim(Origin::signed(1), proof.clone(), 2));
        assert_eq!(
            Proofs::<Test>::get(&proof),
            Some((2, <frame_system::Pallet<Test>>::block_number())),
        );
    });
}

#[test]
fn transfer_claim_failed_when_no_such_proof() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2];
        assert_noop!(
            PoeModule::transfer_claim(Origin::signed(1), proof, 2),
            Error::<Test>::NoSuchProof,
        );
    });
}

#[test]
fn transfer_claim_failed_when_not_proof_owner() {
    new_test_ext().execute_with(|| {
        let proof = vec![1, 2];
        let _ = PoeModule::create_claim(Origin::signed(1), proof.clone());
        assert_noop!(
            PoeModule::transfer_claim(Origin::signed(2), proof, 3), 
            Error::<Test>::NotProofOwner,
        );
    });
}