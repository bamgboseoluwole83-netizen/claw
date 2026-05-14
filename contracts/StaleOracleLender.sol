// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

interface IOracle {
    function latestAnswer() external view returns (uint256);
}

contract StaleOracleLender {
    IOracle public oracle;
    mapping(address => uint256) public deposits;
    mapping(address => uint256) public borrows;
    bool public breached;

    constructor(address _oracle) {
        oracle = IOracle(_oracle);
    }

    function deposit() external payable {
        deposits[msg.sender] += msg.value;
    }

    function borrow(uint256 amount) external {
        uint256 price = oracle.latestAnswer();
        uint256 maxBorrow = (deposits[msg.sender] * price) / 1e18;
        require(amount <= maxBorrow, "borrow exceeds collateral value");
        borrows[msg.sender] += amount;
        payable(msg.sender).transfer(amount);
    }

    function getHealth(address user) external view returns (uint256) {
        if (borrows[user] == 0) return type(uint256).max;
        uint256 price = oracle.latestAnswer();
        uint256 collateralValue = (deposits[user] * price) / 1e18;
        return (collateralValue * 100) / borrows[user];
    }
}
