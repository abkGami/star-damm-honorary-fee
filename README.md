# DAMM v2 Honorary Quote-Only Fee Position

A standalone Anchor-compatible module for creating and managing honorary DAMM v2 LP positions that accrue fees exclusively in quote tokens, with permissionless 24-hour distribution cranks to investors and creators.

## Overview

This module implements Star's requirements for a fee distribution system where:

- **Quote-only fees**: Honorary positions only accrue fees in the quote mint
- **Program ownership**: Fee positions are owned by program PDAs
- **24-hour distribution**: Permissionless crank distributes fees once per 24 hours
- **Pro-rata distribution**: Fees are distributed proportionally to still-locked investor amounts
- **Creator remainder**: Complement goes to creator after investor distribution

## Architecture

### Core Components

1. **Honorary Position**: DAMM v2 LP position owned by program PDA that accrues quote-only fees
2. **Policy PDA**: Configuration including fee shares, caps, and mint addresses
3. **Progress PDA**: Tracks distribution state, pagination, and daily windows
4. **Treasury**: Program-owned ATA for quote mint fee accumulation

### PDA Derivation

```rust
// Position owner PDA
seeds: [VAULT_SEED, vault, "investor_fee_pos_owner"]

// Policy configuration PDA
seeds: [VAULT_SEED, vault, "policy"]

// Progress tracking PDA
seeds: [VAULT_SEED, vault, "progress"]

// Treasury ATA
authority: position_owner_pda
mint: quote_mint
```

## Instructions

### `initialize_honorary_position`

Creates the honorary fee position with quote-only validation.

**Accounts:**

```rust
pub struct InitializeHonoraryPosition<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub vault: UncheckedAccount<'info>, // PDA seed
    pub pool: UncheckedAccount<'info>, // cp-amm pool
    pub quote_mint: Account<'info, Mint>,
    pub base_mint: Account<'info, Mint>,
    pub creator_quote_ata: Account<'info, TokenAccount>,
    pub position_owner_pda: UncheckedAccount<'info>, // [vault, "investor_fee_pos_owner"]
    #[account(init)] pub policy: Account<'info, PolicyState>,
    #[account(init)] pub progress: Account<'info, ProgressState>,
    #[account(init)] pub treasury: Account<'info, TokenAccount>,
    #[account(mut)] pub position: UncheckedAccount<'info>, // Created via cp-amm CPI
    pub cp_amm_program: UncheckedAccount<'info>,
    // ... system programs
}
```

**Parameters:**

- `investor_fee_share_bps: u16` - Investor fee share (0-10000 basis points)
- `daily_cap: u64` - Optional daily distribution cap (0 = no cap)
- `min_payout_lamports: u64` - Minimum payout threshold
- `total_investor_allocation: u64` - Total Y0 allocation for locked percentage calculation

**Validation:**

- Pool configuration must guarantee quote-only fee accrual
- Fee share must be ≤ 10000 basis points
- Creator ATA must match quote mint

### `distribute_fees`

Permissionless crank for claiming and distributing fees.

**Accounts:**

```rust
pub struct DistributeFees<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub vault: UncheckedAccount<'info>,
    pub policy: Account<'info, PolicyState>,
    #[account(mut)] pub progress: Account<'info, ProgressState>,
    pub position_owner_pda: UncheckedAccount<'info>,
    #[account(mut)] pub position: UncheckedAccount<'info>,
    #[account(mut)] pub treasury: Account<'info, TokenAccount>,
    #[account(mut)] pub creator_quote_ata: Account<'info, TokenAccount>,
    pub cp_amm_program: UncheckedAccount<'info>,
    pub streamflow_program: UncheckedAccount<'info>,
    // ... system programs
}
```

**Remaining Accounts (Investor Pages):**
Each investor requires 2 accounts in sequence:

1. Streamflow stream account (for reading locked amount)
2. Investor quote token ATA (for distribution)

**Parameters:**

- `page_size: u64` - Number of investors to process in this call

**Behavior:**

1. **New Day Check**: If 24h elapsed, claims fees from honorary position
2. **Investor Distribution**: Distributes pro-rata based on locked amounts
3. **Creator Payout**: On final page, sends remainder to creator
4. **Pagination**: Supports multiple calls to process all investors

## Fee Distribution Logic

### Locked Percentage Calculation

```rust
Y0 = total_investor_allocation; // From TGE
locked_total(t) = sum of still-locked across all investors;
f_locked(t) = locked_total(t) / Y0; // Percentage still locked
eligible_investor_share_bps = min(investor_fee_share_bps, floor(f_locked(t) * 10000));
```

### Pro-Rata Distribution

```rust
investor_fee_quote = floor(claimed_quote * eligible_investor_share_bps / 10000);
weight_i(t) = locked_i(t) / locked_total(t); // Investor weight
payout_i = floor(investor_fee_quote * weight_i(t)); // Individual payout
```

### Daily Caps and Dust Handling

- **Daily Cap**: `min(calculated_amount, daily_cap - already_distributed)`
- **Dust Threshold**: Amounts below `min_payout_lamports` are carried forward
- **Remainder**: `claimed_quote - total_distributed_to_investors` goes to creator

## Error Codes

| Code | Name                      | Description                           |
| ---- | ------------------------- | ------------------------------------- |
| 6000 | `BaseFeesDetected`        | Position would accrue base token fees |
| 6001 | `InvalidTokenOrder`       | Quote mint not properly identified    |
| 6002 | `CooldownNotElapsed`      | 24-hour period not yet passed         |
| 6003 | `DailyCapExceeded`        | Distribution exceeds daily cap        |
| 6004 | `BelowMinPayout`          | Amount below minimum threshold        |
| 6005 | `NoLockedTokens`          | No locked tokens for distribution     |
| 6006 | `InvalidStreamAccount`    | Streamflow stream account invalid     |
| 6007 | `ArithmeticOverflow`      | Math operation overflow               |
| 6008 | `DistributionComplete`    | Day already complete                  |
| 6009 | `InvalidPaginationCursor` | Pagination cursor out of bounds       |
| 6010 | `PoolNotInitialized`      | Pool account not properly initialized |
| 6011 | `InvalidPositionOwner`    | Position not owned by program PDA     |
| 6012 | `BaseFeesInClaim`         | Base fees detected during claim       |
| 6013 | `InvalidQuoteMint`        | Wrong quote mint for vault            |
| 6014 | `InvalidTreasury`         | Treasury ATA invalid or not found     |

## Events

### `HonoraryPositionInitialized`

```rust
pub struct HonoraryPositionInitialized {
    pub vault: Pubkey,
    pub position_owner_pda: Pubkey,
    pub quote_mint: Pubkey,
    pub position: Pubkey,
    pub timestamp: i64,
}
```

### `QuoteFeesClaimed`

```rust
pub struct QuoteFeesClaimed {
    pub vault: Pubkey,
    pub amount_claimed: u64,
    pub quote_mint: Pubkey,
    pub timestamp: i64,
}
```

### `InvestorPayoutPage`

```rust
pub struct InvestorPayoutPage {
    pub vault: Pubkey,
    pub page_start: u64,
    pub page_end: u64,
    pub total_distributed: u64,
    pub investor_count: u64,
    pub timestamp: i64,
}
```

### `CreatorPayoutDayClosed`

```rust
pub struct CreatorPayoutDayClosed {
    pub vault: Pubkey,
    pub creator_amount: u64,
    pub total_claimed_today: u64,
    pub total_distributed_to_investors: u64,
    pub timestamp: i64,
}
```

## Integration Guide

### 1. Deploy Program

```bash
anchor build
anchor deploy --provider.cluster <cluster>
```

### 2. Initialize Honorary Position

```typescript
const tx = await program.methods
  .initializeHonoraryPosition(
    7500, // 75% to investors
    new BN(1000000), // 1M daily cap
    new BN(1000), // 0.001 min payout
    new BN(10000000) // 10M total allocation
  )
  .accounts({
    payer: payer.publicKey,
    vault: vaultKeypair.publicKey,
    pool: poolAccount.publicKey,
    quoteMint: quoteMintAddress,
    baseMint: baseMintAddress,
    creatorQuoteAta: creatorAtaAddress,
    // ... other accounts
  })
  .rpc();
```

### 3. Set Up Periodic Distribution

```typescript
// Call once per day after 24h cooldown
const investorAccounts = investors.flatMap((inv) => [
  { pubkey: inv.streamAccount, isWritable: false, isSigner: false },
  { pubkey: inv.quoteAta, isWritable: true, isSigner: false },
]);

const tx = await program.methods
  .distributeFees(new BN(50)) // Process 50 investors per page
  .accounts({
    vault: vaultKeypair.publicKey,
    // ... other accounts
  })
  .remainingAccounts(investorAccounts)
  .rpc();
```

### 4. Handle Pagination

```typescript
const totalInvestors = investors.length;
const pageSize = 50;
const totalPages = Math.ceil(totalInvestors / pageSize);

for (let page = 0; page < totalPages; page++) {
  const start = page * pageSize;
  const end = Math.min(start + pageSize, totalInvestors);
  const pageInvestors = investors.slice(start, end);

  const remainingAccounts = pageInvestors.flatMap((inv) => [
    { pubkey: inv.streamAccount, isWritable: false, isSigner: false },
    { pubkey: inv.quoteAta, isWritable: true, isSigner: false },
  ]);

  await program.methods
    .distributeFees(new BN(pageInvestors.length))
    .accounts({
      /* accounts */
    })
    .remainingAccounts(remainingAccounts)
    .rpc();
}
```

## Protocol Invariants

1. **Quote-Only**: Honorary position MUST only accrue quote token fees
2. **24-Hour Gate**: Distribution can only start after 24h from last distribution
3. **Deterministic Math**: All calculations use floor division for determinism
4. **Base Fee Rejection**: Any base token fees cause immediate failure
5. **PDA Ownership**: Honorary position owned by program PDA only
6. **Idempotent Pages**: Re-running pages within same day is safe
7. **Conservation**: `claimed = distributed_to_investors + creator_remainder + dust`

## Testing

### Unit Tests

```bash
anchor test
```

### Integration Tests

```bash
npm run test:bankrun
```

### Test Scenarios Covered

- ✅ Quote-only position initialization
- ✅ Base fee detection and rejection
- ✅ 24-hour cooldown enforcement
- ✅ Pro-rata distribution math
- ✅ Pagination and resumability
- ✅ Daily caps and dust handling
- ✅ Edge cases (all unlocked, zero amounts)
- ✅ Error conditions and recovery

## Security Considerations

1. **Quote-Only Enforcement**: Critical validation before position creation
2. **PDA Authority**: Only program can control honorary position
3. **Streamflow Validation**: Verify stream accounts before reading locked amounts
4. **Math Safety**: All arithmetic operations check for overflow
5. **Re-entrancy**: State updates occur before external calls
6. **Access Control**: No privileged operations beyond initial setup

## Deployment Checklist

- [ ] Program deployed and verified
- [ ] cp-amm program integration tested
- [ ] Streamflow program integration tested
- [ ] Quote-only validation working on target pools
- [ ] 24-hour timing tested with real clock
- [ ] Multi-page distribution tested
- [ ] Error handling verified
- [ ] Events emitted correctly
- [ ] Off-chain monitoring set up

## License

This project is licensed under the MIT License.
