// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

contract MockOracle {
    uint256 public price;

    constructor() {
        price = 1_000_000_000_000_000_000_000_000_000_000_000_000; // 1:1 ETH:USD (18 decimals)
    }

    function setPrice(uint256 _price) external {
        price = _price;
    }

    function latestAnswer() external view returns (uint256) {
        return price;
    }
}
