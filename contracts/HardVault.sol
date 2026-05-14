// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title HardVault — Boss-level vulnerable vault
 *
 * The bug: oracle pricing mismatch between deposit & liquidation paths.
 *
 * Deposit path uses SPOT price from a Uniswap V2 pair.
 * Liquidation path uses a SEPARATE pricing function that computes
 * collateral value INCORRECTLY (uses token1 reserve for both tokens).
 *
 * An attacker can:
 *   1. Manipulate the pool price upward via swap
 *   2. Deposit cheap token A as collateral (valued at inflated price)
 *   3. Borrow token B against inflated collateral
 *   4. The liquidation path CANNOT catch them because it also
 *      uses the manipulated price but with inverse computation
 *   5. Repeat until drained
 *
 * Additionally, the vault has a "fee" that accrues as shares but
 * the share calculation rounds in favor of the depositor during
 * inflation (no donation guard).
 */

interface IERC20 {
    function transferFrom(address, address, uint256) external returns (bool);
    function transfer(address, uint256) external returns (bool);
    function balanceOf(address) external view returns (uint256);
}

interface IUniswapV2Pair {
    function getReserves() external view returns (uint112, uint112, uint32);
    function token0() external view returns (address);
    function token1() external view returns (address);
}

contract HardVault {
    IUniswapV2Pair public pair;
    IERC20 public tokenA; // collateral
    IERC20 public tokenB; // borrow

    mapping(address => uint256) public collateralOf;
    mapping(address => uint256) public debtOf;
    mapping(address => uint256) public depositedAt;

    uint256 public totalCollateral;
    uint256 public constant LIQ_THRESHOLD = 75; // 75% LTV

    constructor(address _pair, address _tokenA, address _tokenB) {
        pair = IUniswapV2Pair(_pair);
        tokenA = IERC20(_tokenA);
        tokenB = IERC20(_tokenB);
    }

    // ─── DEPOSIT PATH: uses CORRECT price (reserve1/reserve0) ───
    function getPriceTokenA() public view returns (uint256) {
        (uint112 r0, uint112 r1,) = pair.getReserves();
        if (pair.token0() == address(tokenA)) {
            return uint256(r1) * 1e18 / uint256(r0); // tokenA = token0 → price = r1/r0
        }
        return uint256(r0) * 1e18 / uint256(r1); // tokenA = token1 → price = r0/r1
    }

    // ─── LIQUIDATION PATH: uses WRONG price (reserve1/reserve1 = 1) ───
    // BUG: both branches compute reserve1/reserve1 when tokenA == token0,
    // effectively pricing tokenA at 1 wei regardless of reserves
    function getPriceTokenABroken() public view returns (uint256) {
        (uint112 r0, uint112 r1,) = pair.getReserves();
        if (pair.token0() == address(tokenA)) {
            return uint256(r0) * 1e18 / uint256(r1); // SHOULD BE r1/r0
        }
        return uint256(r0) * 1e18 / uint256(r1); // correct for tokenA == token1
    }

    // ─── DEPOSIT ───
    function deposit(uint256 amount) external {
        require(tokenA.transferFrom(msg.sender, address(this), amount), "deposit failed");
        uint256 price = getPriceTokenA();
        uint256 depositValue = amount * price / 1e18;
        collateralOf[msg.sender] += depositValue;
        totalCollateral += depositValue;
        depositedAt[msg.sender] = block.timestamp;
    }

    // ─── WITHDRAW ───
    function withdraw(uint256 collateralValue) external {
        require(collateralOf[msg.sender] >= collateralValue, "insufficient collateral");
        uint256 price = getPriceTokenA();
        uint256 amount = collateralValue * 1e18 / price;
        collateralOf[msg.sender] -= collateralValue;
        totalCollateral -= collateralValue;
        require(tokenA.transfer(msg.sender, amount), "withdraw failed");
    }

    // ─── BORROW: uses getPriceTokenA() (correct) ───
    function borrow(uint256 amountB) external {
        uint256 price = getPriceTokenA();
        uint256 maxDebt = collateralOf[msg.sender] * LIQ_THRESHOLD / 100;
        uint256 debtValue = amountB * 1e18 / price; // normalize
        require(debtOf[msg.sender] + debtValue <= maxDebt, "over limit");
        debtOf[msg.sender] += debtValue;
        require(tokenB.transfer(msg.sender, amountB), "borrow failed");
    }

    // ─── REPAY ───
    function repay(uint256 amountB) external {
        require(tokenB.transferFrom(msg.sender, address(this), amountB), "repay failed");
        uint256 price = getPriceTokenA();
        uint256 debtValue = amountB * 1e18 / price;
        debtOf[msg.sender] -= debtValue;
    }

    // ─── LIQUIDATE: uses getPriceTokenABroken() (WRONG!) ───
    // Uses broken price → can only liquidate when collateral is
    // truly worthless, not when position is actually underwater
    function liquidate(address user) external {
        uint256 price = getPriceTokenABroken(); // <-- BUG: broken price
        uint256 collatValue = collateralOf[user] * price / 1e18;

        // If user borrowed 100 tokenB and price(A) = 0.5 tokenB,
        // debtValue should be 200. But broken price may return 1,
        // making collatValue = collateralOf[user].
        // So health = collatValue * 100 / debtValue can be > 100
        // → liquidation always fails when it should succeed
        uint256 debtValue = debtOf[user];
        uint256 health = collatValue * 100 / debtValue;
        require(health < 100, "position healthy");

        uint256 reward = collateralOf[user] * 95 / 100;
        collateralOf[user] = 0;
        debtOf[user] = 0;
        totalCollateral -= collateralOf[user];
        require(tokenA.transfer(msg.sender, reward), "liq failed");
    }

    // ─── VAULT FEE BUG: feeOnDeposit rounds in depositor's favor ───
    // During share inflation (donation), new depositors get more shares
    // than they should because the calculation rounds UP for shares
    uint256 public totalShares;
    mapping(address => uint256) public shares;

    function vaultDeposit(uint256 amountA) external {
        tokenA.transferFrom(msg.sender, address(this), amountA);
        uint256 _shares = totalShares == 0
            ? amountA
            : amountA * totalShares / tokenA.balanceOf(address(this)); // BUG: should be totalAssets, not balance
        shares[msg.sender] += _shares;
        totalShares += _shares;
    }

    function vaultWithdraw(uint256 _shares) external {
        uint256 amount = _shares * (tokenA.balanceOf(address(this)) - 1) / totalShares; // BUG: integer truncation favors attacker
        shares[msg.sender] -= _shares;
        totalShares -= _shares;
        tokenA.transfer(msg.sender, amount);
    }
}
