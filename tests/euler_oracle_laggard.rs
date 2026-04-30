use alloy_primitives::{Address, U256};
use alloy_sol_types::{sol, SolCall};
use revm::Database;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode, SpecId, TransactTo};
use revm::Evm;

sol! {
    function deposit() external payable;
    function borrow(uint256 amount) external;
}

/// Reproduce the Euler Oracle Laggard pattern:
/// 1. Oracle reports a stale, high price.
/// 2. Attacker deposits a small collateral.
/// 3. Attacker borrows a huge amount based on the stale price.
/// 4. Oracle updates to the real (much lower) price.
/// 5. The loan is now severely under‑collateralised.
#[test]
fn test_euler_style_oracle_laggard_detected() {
    let oracle_hex = std::fs::read_to_string("mocks/StaleOracle.hex")
        .unwrap().trim().to_string();
    let lender_hex = std::fs::read_to_string("mocks/StaleLender.hex")
        .unwrap().trim().to_string();

    let oracle_code = Bytecode::new_raw(hex::decode(&oracle_hex).unwrap().into());
    let lender_code = Bytecode::new_raw(hex::decode(&lender_hex).unwrap().into());

    let mut db = CacheDB::new(EmptyDB::new());
    let oracle_addr = Address::repeat_byte(0x50);
    let lender_addr = Address::repeat_byte(0x60);
    let attacker = Address::repeat_byte(0xde);

    db.insert_account_info(oracle_addr, AccountInfo {
        balance: U256::ZERO, nonce: 1,
        code: Some(oracle_code), code_hash: Default::default(),
    });
    db.insert_account_info(lender_addr, AccountInfo {
        balance: U256::from(1000_000_000_000_000_000_000u128),
        nonce: 1,
        code: Some(lender_code), code_hash: Default::default(),
    });

    // Set the oracle address in the lender's storage (slot 2)
    let mut oracle_bytes = [0u8; 32];
    oracle_bytes[12..32].copy_from_slice(oracle_addr.as_ref());
    db.insert_account_storage(lender_addr, U256::from(2), U256::from_be_bytes(oracle_bytes)).unwrap();

    // Stale high price (100 ETH)
    let stale_price = U256::from(100_000_000_000_000_000_000u128);
    db.insert_account_storage(oracle_addr, U256::from(0), stale_price).unwrap();

    // Attacker deposits 1 ETH collateral
    let attacker_collateral = U256::from(1_000_000_000_000_000_000u128);
    db.insert_account_info(attacker, AccountInfo {
        balance: attacker_collateral * U256::from(100),
        nonce: 0,
        code: None,
        code_hash: Default::default(),
    });
    let deposit_calldata = depositCall::new(()).abi_encode();
    {
        let mut evm = Evm::builder()
            .with_db(&mut db)
            .with_spec_id(SpecId::LATEST)
            .build();
        *evm.tx_mut() = revm::primitives::TxEnv {
            caller: attacker,
            transact_to: TransactTo::Call(lender_addr),
            data: alloy_primitives::Bytes::from(deposit_calldata),
            value: attacker_collateral,
            ..Default::default()
        };
        evm.transact_commit().ok();
    }

    // Borrow 50 ETH at the stale price
    let borrow_amount = U256::from(50) * U256::from(10).pow(U256::from(18));
    let borrow_calldata = borrowCall::new((borrow_amount,)).abi_encode();
    {
        let mut evm = Evm::builder()
            .with_db(&mut db)
            .with_spec_id(SpecId::LATEST)
            .build();
        *evm.tx_mut() = revm::primitives::TxEnv {
            caller: attacker,
            transact_to: TransactTo::Call(lender_addr),
            data: alloy_primitives::Bytes::from(borrow_calldata),
            value: U256::ZERO,
            ..Default::default()
        };
        evm.transact_commit().ok();
    }

    // Oracle updates to true price (1 ETH)
    let true_price = U256::from(1_000_000_000_000_000_000u128);
    db.insert_account_storage(oracle_addr, U256::from(0), true_price).unwrap();

    // Check under‑collateralisation
    let coll_slot = mapping_slot(attacker, U256::from(0));
    let loan_slot = mapping_slot(attacker, U256::from(1));
    let collateral = db.storage(lender_addr, coll_slot).unwrap_or(U256::ZERO);
    let loan = db.storage(lender_addr, loan_slot).unwrap_or(U256::ZERO);
    let safe_max = collateral * true_price / U256::from(10).pow(U256::from(18));

    println!("Loan: {loan}");
    println!("Collateral: {collateral}");
    println!("Safe max: {safe_max}");

    assert!(
        loan > safe_max,
        "Euler‑style Oracle Laggard should leave the protocol under‑collateralised"
    );
    println!("✅ Euler‑style Oracle Laggard successfully detected!");
}

fn mapping_slot(user: Address, base_slot: U256) -> U256 {
    let mut preimage = [0u8; 64];
    preimage[12..32].copy_from_slice(user.as_ref());
    preimage[32..64].copy_from_slice(&base_slot.to_be_bytes::<32>());
    let key = alloy_primitives::keccak256(preimage);
    U256::from_be_bytes(key.0)
}
