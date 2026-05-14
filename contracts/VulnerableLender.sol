// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title VulnerableLender
 * @notice Intentionally vulnerable lending protocol for testing web3-destroyer
 *
 * Vulnerabilities:
 * 1. Oracle manipulation: getReserves() from single Uniswap V2 pair, no TWAP
 * 2. No reentrancy guard on withdraw()
 * 3. Missing access control on setOracle()
 * 4. Integer truncation in collateral calculation
 * 5. Flash loan token balance assumption
 */

interface IERC20 {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function transferFrom(address, address, uint256) external returns (bool);
}

interface IUniswapV2Pair {
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    function token0() external view returns (address);
    function token1() external view returns (address);
    function swap(uint256, uint256, address, bytes calldata) external;
}

contract VulnerableLender {
    IUniswapV2Pair public oraclePool;
    IERC20 public collateralToken;
    IERC20 public borrowToken;

    mapping(address => uint256) public deposits;
    mapping(address => uint256) public borrowed;

    uint256 public constant LIQUIDATION_THRESHOLD = 80;
    uint256 public totalDeposits;

    event Deposited(address indexed user, uint256 amount);
    event Withdrawn(address indexed user, uint256 amount);
    event Borrowed(address indexed user, uint256 amount);
    event Repaid(address indexed user, uint256 amount);
    event Liquidated(address indexed target, address indexed liquidator, uint256 amount);

    constructor(address _oraclePool, address _collateralToken, address _borrowToken) {
        oraclePool = IUniswapV2Pair(_oraclePool);
        collateralToken = IERC20(_collateralToken);
        borrowToken = IERC20(_borrowToken);
    }

    // ─── VULN 3: No access control ───
    function setOracle(address newPool) external {
        oraclePool = IUniswapV2Pair(newPool);
    }

    // ─── VULN 1: Oracle manipulation ───
    function getCollateralPrice() public view returns (uint256) {
        (uint112 reserve0, uint112 reserve1, ) = oraclePool.getReserves();
        if (address(collateralToken) == oraclePool.token0()) {
            return uint256(reserve1) * 1e18 / uint256(reserve0);
        }
        return uint256(reserve0) * 1e18 / uint256(reserve1);
    }

    function deposit(uint256 amount) external {
        require(collateralToken.transferFrom(msg.sender, address(this), amount), "transfer failed");
        deposits[msg.sender] += amount;
        totalDeposits += amount;
        emit Deposited(msg.sender, amount);
    }

    // ─── VULN 2: No reentrancy guard ───
    function withdraw(uint256 amount) external {
        require(deposits[msg.sender] >= amount, "insufficient deposit");
        deposits[msg.sender] -= amount;
        totalDeposits -= amount;
        require(collateralToken.transfer(msg.sender, amount), "transfer failed");
        emit Withdrawn(msg.sender, amount);
    }

    function borrow(uint256 amount) external {
        uint256 price = getCollateralPrice();
        uint256 maxBorrow = (deposits[msg.sender] * price / 1e18) * LIQUIDATION_THRESHOLD / 100;
        // ─── VULN 4: Integer truncation ───
        require(borrowed[msg.sender] + amount <= maxBorrow, "exceeds max borrow");
        borrowed[msg.sender] += amount;
        require(borrowToken.transfer(msg.sender, amount), "borrow transfer failed");
        emit Borrowed(msg.sender, amount);
    }

    function repay(uint256 amount) external {
        require(borrowToken.transferFrom(msg.sender, address(this), amount), "repay transfer failed");
        borrowed[msg.sender] -= amount;
        emit Repaid(msg.sender, amount);
    }

    // ─── VULN 5: Flash loan assumption ───
    function flashLoan(uint256 amount, address target, bytes calldata data) external {
        uint256 before = borrowToken.balanceOf(address(this));
        require(before >= amount, "insufficient liquidity");

        require(borrowToken.transfer(target, amount), "flash transfer failed");
        (bool success, ) = target.call(data);
        require(success, "flash callback failed");

        uint256 after = borrowToken.balanceOf(address(this));
        require(after >= before + amount * 10 / 1000, "flash loan not repaid");
    }

    function getAccountHealth(address user) external view returns (uint256) {
        if (borrowed[user] == 0) return type(uint256).max;
        uint256 price = getCollateralPrice();
        uint256 collateralValue = deposits[user] * price / 1e18;
        return collateralValue * 100 / borrowed[user];
    }

    function liquidate(address user) external {
        uint256 health = this.getAccountHealth(user);
        require(health < 100, "position is healthy");

        uint256 repayAmount = borrowed[user];
        uint256 collateral = deposits[user];
        uint256 liquidatorAmount = collateral * 95 / 100;

        require(borrowToken.transferFrom(msg.sender, address(this), repayAmount), "repay failed");
        borrowed[user] = 0;
        deposits[user] = 0;
        totalDeposits -= collateral;

        require(collateralToken.transfer(msg.sender, liquidatorAmount), "collateral transfer failed");
        emit Liquidated(user, msg.sender, liquidatorAmount);
    }
}
