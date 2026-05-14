// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

interface IYieldSource {
    function pricePerShare() external view returns (uint256);
}

contract CreamLending {
    IYieldSource public oracle;
    mapping(address => uint256) public deposits;
    mapping(address => uint256) public borrows;

    constructor(address _oracle) {
        oracle = IYieldSource(_oracle);
    }

    function deposit() external payable {
        deposits[msg.sender] += msg.value;
    }

    function borrow(uint256 amount) external {
        uint256 price = oracle.pricePerShare();
        uint256 maxBorrow = (deposits[msg.sender] * price) / 1e18;
        require(amount <= maxBorrow, "borrow exceeds collateral");
        require(amount <= address(this).balance, "insufficient liquidity");
        borrows[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }
}
