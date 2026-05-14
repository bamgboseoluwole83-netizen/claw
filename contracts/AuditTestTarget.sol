// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

contract AuditTestTarget {
    mapping(address => uint256) public balances;
    uint256 public totalPoolFunds;

    function withdrawAll() public {
        uint256 total = address(this).balance;
        (bool success, ) = msg.sender.call{value: total}("");
        require(success, "Transfer failed");
        totalPoolFunds = 0;
    }

    function getAssetPrice(bool isLiquidationCheck) public pure returns (uint256) {
        if (isLiquidationCheck) { return 1; }
        return 1000;
    }

    function deposit() public payable {
        balances[msg.sender] += msg.value;
        totalPoolFunds += msg.value;
        if (msg.value % 10 > 0) { balances[msg.sender] += 1; }
    }
    receive() external payable {}
}
