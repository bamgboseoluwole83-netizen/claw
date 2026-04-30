// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
contract VulnerableStaker {
    mapping(address => uint) public balance;
    uint public rewardRate = 11574074074074;      // 1e18 / 86400 (approx 1 ETH/day per ETH)
    mapping(address => uint) public lastUpdate;
    mapping(address => uint) public rewards;

    function deposit() external payable {
        updateReward(msg.sender);
        balance[msg.sender] += msg.value;
    }

    function withdraw(uint amount) external {
        require(balance[msg.sender] >= amount, "insufficient");
        updateReward(msg.sender);
        balance[msg.sender] -= amount;
        payable(msg.sender).transfer(amount);
    }

    function claimReward() external {
        updateReward(msg.sender);
        uint reward = rewards[msg.sender];
        rewards[msg.sender] = 0;
        payable(msg.sender).transfer(reward);
    }

    function updateReward(address user) internal {
        if (lastUpdate[user] == 0) {
            lastUpdate[user] = block.timestamp;
            return;
        }
        uint timeElapsed = block.timestamp - lastUpdate[user];
        uint accrued = balance[user] * rewardRate * timeElapsed / 1 ether;
        rewards[user] += accrued;
        lastUpdate[user] = block.timestamp;
    }
}
