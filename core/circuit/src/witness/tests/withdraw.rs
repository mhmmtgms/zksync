// External deps
use bigdecimal::BigDecimal;
use crypto_exports::franklin_crypto::bellman::pairing::bn256::Bn256;
// Workspace deps
use models::node::{operations::WithdrawOp, Address};
use plasma::state::CollectedFee;
// Local deps
use crate::witness::{
    tests::test_utils::{
        corrupted_input_test_scenario, generic_test_scenario, incorrect_op_test_scenario,
        WitnessTestAccount,
    },
    utils::SigDataInput,
    withdraw::WithdrawWitness,
};

#[test]
#[ignore]
fn test_withdraw() {
    // Test vector of (initial_balance, transfer_amount, fee_amount).
    let test_vector = vec![
        (10, 7, 3),                // Basic transfer
        (0, 0, 0),                 // Zero transfer
        (std::u64::MAX, 1, 1),     // Small transfer from rich account,
        (std::u64::MAX, 10000, 1), // Big transfer from rich account (too big values can't be used, since they're not packable),
        (std::u64::MAX, 1, 10000), // Very big fee
    ];

    for (initial_balance, transfer_amount, fee_amount) in test_vector {
        // Input data.
        let accounts = vec![WitnessTestAccount::new(1, initial_balance)];
        let account = &accounts[0];
        let withdraw_op = WithdrawOp {
            tx: account
                .zksync_account
                .sign_withdraw(
                    0,
                    "",
                    BigDecimal::from(transfer_amount),
                    BigDecimal::from(fee_amount),
                    &Address::zero(),
                    None,
                    true,
                )
                .0,
            account_id: account.id,
        };

        // Additional data required for performing the operation.
        let input =
            SigDataInput::from_withdraw_op(&withdraw_op).expect("SigDataInput creation failed");

        generic_test_scenario::<WithdrawWitness<Bn256>, _>(
            &accounts,
            withdraw_op,
            input,
            |plasma_state, op| {
                let (fee, _) = plasma_state
                    .apply_withdraw_op(&op)
                    .expect("transfer should be success");
                vec![fee]
            },
        );
    }
}

/// Checks that corrupted signature data leads to unsatisfied constraints in circuit.
#[test]
#[ignore]
fn corrupted_ops_input() {
    // Incorrect signature data will lead to `op_valid` constraint failure.
    // See `circuit.rs` for details.
    const EXPECTED_PANIC_MSG: &str = "op_valid is true";

    // Legit input data.
    let accounts = vec![WitnessTestAccount::new(1, 10)];
    let account = &accounts[0];
    let withdraw_op = WithdrawOp {
        tx: account
            .zksync_account
            .sign_withdraw(
                0,
                "",
                BigDecimal::from(7),
                BigDecimal::from(3),
                &Address::zero(),
                None,
                true,
            )
            .0,
        account_id: account.id,
    };

    // Additional data required for performing the operation.
    let input = SigDataInput::from_withdraw_op(&withdraw_op).expect("SigDataInput creation failed");

    // Test vector with values corrupted one by one.
    let test_vector = input.corrupted_variations();

    for input in test_vector {
        corrupted_input_test_scenario::<WithdrawWitness<Bn256>, _>(
            &accounts,
            withdraw_op.clone(),
            input,
            EXPECTED_PANIC_MSG,
            |plasma_state, op| {
                let (fee, _) = plasma_state
                    .apply_withdraw_op(&op)
                    .expect("transfer should be success");
                vec![fee]
            },
        );
    }
}

/// Checks that executing a withdraw operation with incorrect
/// data (account `from` ID) results in an error.
#[test]
#[ignore]
fn test_incorrect_withdraw_account_from() {
    const TOKEN_ID: u16 = 0;
    const INITIAL_BALANCE: u64 = 10;
    const TOKEN_AMOUNT: u64 = 7;
    const FEE_AMOUNT: u64 = 3;

    // Operation is not valid, since `from` ID is different from the tx body.
    const ERR_MSG: &str = "op_valid is true/enforce equal to one";

    let incorrect_from_account = WitnessTestAccount::new(3, INITIAL_BALANCE);

    // Input data: transaction is signed by an incorrect account (address of account
    // and ID of the `from` accounts differ).
    let accounts = vec![WitnessTestAccount::new(1, INITIAL_BALANCE)];
    let account_from = &accounts[0];
    let withdraw_op = WithdrawOp {
        tx: incorrect_from_account
            .zksync_account
            .sign_withdraw(
                TOKEN_ID,
                "",
                BigDecimal::from(TOKEN_AMOUNT),
                BigDecimal::from(FEE_AMOUNT),
                &Address::zero(),
                None,
                true,
            )
            .0,
        account_id: account_from.id,
    };

    let input = SigDataInput::from_withdraw_op(&withdraw_op).expect("SigDataInput creation failed");

    incorrect_op_test_scenario::<WithdrawWitness<Bn256>, _>(
        &accounts,
        withdraw_op,
        input,
        ERR_MSG,
        || {
            vec![CollectedFee {
                token: TOKEN_ID,
                amount: FEE_AMOUNT.into(),
            }]
        },
    );
}

/// Checks that executing a withdraw operation with incorrect
/// data (insufficient funds) results in an error.
#[test]
#[ignore]
fn test_incorrect_withdraw_amount() {
    const TOKEN_ID: u16 = 0;
    // Balance check should fail.
    // "balance-fee bits" is message for subtraction check in circuit.
    // For details see `circuit.rs`.
    const ERR_MSG: &str = "balance-fee bits";

    // Test vector of (initial_balance, transfer_amount, fee_amount).
    let test_vector = vec![
        (10, 15, 0), // Withdraw too big
        (10, 7, 4),  // Fee too big
        (0, 1, 1),   // Withdraw from 0 balance
    ];

    for (initial_balance, transfer_amount, fee_amount) in test_vector {
        // Input data: account does not have enough funds.
        let accounts = vec![WitnessTestAccount::new(1, initial_balance)];
        let account_from = &accounts[0];
        let withdraw_op = WithdrawOp {
            tx: account_from
                .zksync_account
                .sign_withdraw(
                    TOKEN_ID,
                    "",
                    BigDecimal::from(transfer_amount),
                    BigDecimal::from(fee_amount),
                    &Address::zero(),
                    None,
                    true,
                )
                .0,
            account_id: account_from.id,
        };

        let input =
            SigDataInput::from_withdraw_op(&withdraw_op).expect("SigDataInput creation failed");

        incorrect_op_test_scenario::<WithdrawWitness<Bn256>, _>(
            &accounts,
            withdraw_op,
            input,
            ERR_MSG,
            || {
                vec![CollectedFee {
                    token: TOKEN_ID,
                    amount: fee_amount.into(),
                }]
            },
        );
    }
}

//     #[test]
//     #[ignore]
//     #[should_panic(expected = "chunk number 0/execute_op/op_valid")]
//     fn test_withdraw_replay() {
//         use testkit::zksync_account::ZksyncAccount;

//         let account_id = 1;
//         let account_duplicate_id = 11;
//         let mut zksync_account = ZksyncAccount::rand();
//         zksync_account.account_id = Some(account_id);
//         let account_address = zksync_account.address;
//         let account = {
//             let mut account = Account::default_with_address(&account_address);
//             account.add_balance(0, &BigDecimal::from(10));
//             account.pub_key_hash = zksync_account.pubkey_hash.clone();
//             account
//         };

//         let (mut plasma_state, mut circuit_account_tree) = test_genesis_plasma_state(vec![
//             (account_id, account.clone()),
//             (account_duplicate_id, account),
//         ]);
//         let fee_account_id = 0;
//         let mut witness_accum = WitnessBuilder::new(&mut circuit_account_tree, fee_account_id, 1);

//         let withdraw_op = WithdrawOp {
//             tx: zksync_account
//                 .sign_withdraw(
//                     0,
//                     "",
//                     BigDecimal::from(7),
//                     BigDecimal::from(3),
//                     &Address::zero(),
//                     None,
//                     true,
//                 )
//                 .0,
//             account_id: account_duplicate_id,
// }
