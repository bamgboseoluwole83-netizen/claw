// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IStaleOracle {
    function price() external view returns (uint);
}

contract StaleLender {
    mapping(address => uint) public collateral;  // slot 0
    mapping(address => uint) public loans;       // slot 1
    IStaleOracle public oracle;                  // slot 2

    constructor(address _oracle) {
        oracle = IStaleOracle(_oracle);
    }

    function deposit() external payable {
        collateral[msg.sender] += msg.value;
    }

    function borrow(uint amount) external {
        uint currentPrice = oracle.price();
        uint maxBorrow = (collateral[msg.sender] * currentPrice) / 1 ether;
        require(amount <= maxBorrow, "exceeds max");
        loans[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
}
