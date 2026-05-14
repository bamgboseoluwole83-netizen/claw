// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// ============================================================
// Challenge: Two-step nonce-gated proxy oracle manipulation
//
// Vulnerability requires:
//   1. Discover oracle address via storage slot probing
//   2. Detect DELEGATECALL → find implementation via slot 0
//   3. Find prepare() and setPrice() write candidates
//   4. Try individually → prepare() doesn't change price,
//      setPrice() REVERTS (nonce is 0)
//   5. Combine pair: prepare() THEN setPrice() → price changes!
//   6. Deposit → borrow → profit
// ============================================================

// --- Oracle Implementation (deployed behind proxy) ---
contract OracleImpl {
    address public implementation;
    address public owner;
    uint256 public price;
    uint256 public nonce;

    // Step 1: Must be called first — increments nonce
    function prepare() external {
        nonce++;
    }

    // Step 2: Consumes nonce to set price
    // Individually: REVERTS because nonce is 0
    // As pair with prepare(): nonce > 0, so succeeds
    function setPrice(uint256 newPrice) external {
        require(nonce > 0, "prepare first (nonce=0)");
        nonce--;
        price = newPrice;
    }

    function getPrice() external view returns (uint256) {
        return price;
    }

    function reset() external {
        price = 0;
        nonce = 0;
    }
}

// --- Proxy (delegates to implementation) ---
contract OracleProxy {
    address public implementation;  // slot 0

    constructor(address _impl) {
        implementation = _impl;
    }

    fallback(bytes calldata data) external payable returns (bytes memory) {
        (bool ok, bytes memory result) = implementation.delegatecall(data);
        require(ok);
        return result;
    }
}

// --- Lending Pool (reads price from slot-0 oracle) ---
contract ChallengeLending {
    address private oracle;  // slot 0 = oracle addr (probed by pipeline)

    mapping(address => uint256) public deposits;

    constructor(address _oracle) {
        oracle = _oracle;
    }

    function deposit() external payable {
        deposits[msg.sender] += msg.value;
    }

    function borrow(uint256 amount) external {
        (bool ok, bytes memory result) = oracle.staticcall(
            abi.encodeWithSignature("getPrice()")
        );
        require(ok, "oracle call failed");
        uint256 price = abi.decode(result, (uint256));
        require(price > 0, "price is zero");

        uint256 maxBorrow = (deposits[msg.sender] * price) / 1e18;
        require(amount <= maxBorrow, "borrow exceeds collateral");
        require(amount <= address(this).balance, "insufficient liquidity");
        deposits[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
}
