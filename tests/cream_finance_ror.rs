use alloy_primitives::{Address, U256};
use revm::Database;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode, SpecId, TransactTo};
use revm::Evm;
use std::sync::Arc;

/// This test simulates the Cream Finance Read‑Only Reentrancy exploit pattern:
/// 1. Inflate the collateral price via a flash loan.
/// 2. Borrow against the inflated price.
/// 3. Repay the flash loan.
/// 4. The protocol is left with bad debt.
///
/// We use simplified mock contracts that mimic the vulnerable logic.
#[tokio::test]
async fn test_cream_style_ror_detected() {
    // Load mock contracts that mirror the vulnerable Cream pattern
    let pool_hex = std::fs::read_to_string("mocks/DirtyPool.hex")
        .unwrap().trim().to_string();
    let lender_hex = std::fs::read_to_string("mocks/LendX.hex")
        .unwrap().trim().to_string();

    let pool_code = Bytecode::new_raw(hex::decode(&pool_hex).unwrap().into());
    let lender_code = Bytecode::new_raw(hex::decode(&lender_hex).unwrap().into());

    let mut db = CacheDB::new(EmptyDB::new());
    let pool_addr = Address::repeat_byte(0x10);
    let lender_addr = Address::repeat_byte(0x20);
    let attacker = Address::repeat_byte(0xde);

    db.insert_account_info(pool_addr, AccountInfo {
        balance: U256::ZERO, nonce: 1,
        code: Some(pool_code), code_hash: Default::default(),
    });
    db.insert_account_info(lender_addr, AccountInfo {
        balance: U256::from(1000_000_000_000_000_000_000u128),
        nonce: 1,
        code: Some(lender_code), code_hash: Default::default(),
    });

    // Set initial pool price = 1 ETH (slot 0)
    let price_slot = U256::from(0);
    db.insert_account_storage(pool_addr, price_slot, U256::from(1_000_000_000_000_000_000u128)).unwrap();

    // Attacker deposits 1 ETH collateral (mapping at slot 0 of lender)
    let coll_slot = mapping_slot(attacker, U256::from(0));
    db.insert_account_storage(lender_addr, coll_slot, U256::from(1_000_000_000_000_000_000u128)).unwrap();

    // ---- Attack sequence ----
    // 1. Inflate pool price to 100 ETH (simulate flash loan swap)
    db.insert_account_storage(pool_addr, price_slot, U256::from(100_000_000_000_000_000_000u128)).unwrap();

    // 2. Borrow 50 ETH
    let borrow_amount = U256::from(50) * U256::from(10).pow(U256::from(18));
    let borrow_selector = alloy_primitives::keccak256("borrow(uint256,address)".as_bytes());
    let mut borrow_calldata = Vec::from(&borrow_selector[..4]);
    borrow_calldata.extend_from_slice(&borrow_amount.to_be_bytes::<32>());
    borrow_calldata.extend_from_slice(&[0u8; 12]);
    borrow_calldata.extend_from_slice(pool_addr.as_ref());

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

    // 3. Restore pool price to 1 ETH (flash loan repaid, pool returns to normal)
    db.insert_account_storage(pool_addr, price_slot, U256::from(1_000_000_000_000_000_000u128)).unwrap();

    // 4. Check if the loan is under‑collateralised
    let loan_slot = mapping_slot(attacker, U256::from(1));
    let loan = db.storage(lender_addr, loan_slot).unwrap_or(U256::ZERO);
    let collateral = db.storage(lender_addr, coll_slot).unwrap();
    let safe_max = collateral * U256::from(1_000_000_000_000_000_000u128) / U256::from(10).pow(U256::from(18));

    println!("Loan: {loan}");
    println!("Collateral: {collateral}");
    println!("Safe max: {safe_max}");

    assert!(
        loan > safe_max,
        "Cream‑style ROR exploit should leave the protocol under‑collateralised"
    );
    println!("✅ Cream‑style ROR successfully detected!");
}

fn mapping_slot(user: Address, base_slot: U256) -> U256 {
    let mut preimage = [0u8; 64];
    preimage[12..32].copy_from_slice(user.as_ref());
    preimage[32..64].copy_from_slice(&base_slot.to_be_bytes::<32>());
    let key = alloy_primitives::keccak256(preimage);
    U256::from_be_bytes(key.0)
}
