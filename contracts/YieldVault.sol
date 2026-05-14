// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

contract YieldVault {
    uint256 public totalShares;

    function pricePerShare() external view returns (uint256) {
        if (totalShares == 0) return 1e18;
        return (address(this).balance * 1e18) / totalShares;
    }

    function donate() external payable {}

    function deposit() external payable {
        totalShares += msg.value;
    }

    receive() external payable {}
}
