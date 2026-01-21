/**
 * Integration tests for Seahorse TypeScript test helpers
 *
 * These tests verify the test utilities work correctly with LiteSVM.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { LiteSVM } from 'litesvm';
import { Keypair, PublicKey, LAMPORTS_PER_SOL, SystemProgram } from '@solana/web3.js';
import {
  createLiteSvm,
  fundedKeypair,
  fundedKeypair10Sol,
  instructionDiscriminator,
  accountDiscriminator,
  anchorInstruction,
  signerMeta,
  writableMeta,
  readonlyMeta,
  executeTx,
  getAccount,
  rentExemptLamports,
  warpToSlot,
  findPda,
  CALCULATOR_SIZE,
  COUNTER_SIZE,
} from './helpers';

describe('LiteSVM Basic Operations', () => {
  let svm: LiteSVM;

  beforeEach(() => {
    svm = createLiteSvm();
  });

  it('should create a LiteSVM instance', () => {
    expect(svm).toBeDefined();
    const blockhash = svm.latestBlockhash();
    expect(blockhash).toBeDefined();
    expect(blockhash.length).toBeGreaterThan(0);
  });

  it('should airdrop lamports to a keypair', () => {
    const keypair = fundedKeypair(svm, LAMPORTS_PER_SOL);
    const account = getAccount(svm, keypair.publicKey);

    expect(account).not.toBeNull();
    expect(Number(account.lamports)).toBe(LAMPORTS_PER_SOL);
  });

  it('should create a 10 SOL funded keypair', () => {
    const keypair = fundedKeypair10Sol(svm);
    const account = getAccount(svm, keypair.publicKey);

    expect(account).not.toBeNull();
    expect(Number(account.lamports)).toBe(10 * LAMPORTS_PER_SOL);
  });
});

describe('Anchor Discriminator Calculation', () => {
  it('should calculate instruction discriminator correctly', () => {
    // The discriminator is sha256("global:<name>")[..8]
    const disc = instructionDiscriminator('initialize');
    expect(disc).toBeInstanceOf(Buffer);
    expect(disc.length).toBe(8);

    // Same name should produce same discriminator
    const disc2 = instructionDiscriminator('initialize');
    expect(disc.equals(disc2)).toBe(true);

    // Different name should produce different discriminator
    const disc3 = instructionDiscriminator('increment');
    expect(disc.equals(disc3)).toBe(false);
  });

  it('should calculate account discriminator correctly', () => {
    // The discriminator is sha256("account:<Name>")[..8]
    const disc = accountDiscriminator('Calculator');
    expect(disc).toBeInstanceOf(Buffer);
    expect(disc.length).toBe(8);

    // Different account should produce different discriminator
    const disc2 = accountDiscriminator('Counter');
    expect(disc.equals(disc2)).toBe(false);
  });
});

describe('Anchor Instruction Building', () => {
  it('should build an anchor instruction with discriminator', () => {
    const programId = Keypair.generate().publicKey;
    const user = Keypair.generate().publicKey;

    const ix = anchorInstruction(
      programId,
      'initialize',
      Buffer.from([]),
      [signerMeta(user)]
    );

    // Instruction should have 8-byte discriminator
    expect(ix.data.length).toBe(8);
    expect(ix.programId.equals(programId)).toBe(true);
    expect(ix.keys.length).toBe(1);
    expect(ix.keys[0].isSigner).toBe(true);
    expect(ix.keys[0].isWritable).toBe(true);
  });

  it('should build instruction with data appended after discriminator', () => {
    const programId = Keypair.generate().publicKey;
    const extraData = Buffer.from([1, 2, 3, 4]);

    const ix = anchorInstruction(
      programId,
      'set_value',
      extraData,
      []
    );

    // Instruction should have 8-byte discriminator + 4 bytes data
    expect(ix.data.length).toBe(12);
  });
});

describe('Account Meta Helpers', () => {
  it('should create signer meta correctly', () => {
    const pubkey = Keypair.generate().publicKey;
    const meta = signerMeta(pubkey);

    expect(meta.pubkey.equals(pubkey)).toBe(true);
    expect(meta.isSigner).toBe(true);
    expect(meta.isWritable).toBe(true);
  });

  it('should create writable meta correctly', () => {
    const pubkey = Keypair.generate().publicKey;
    const meta = writableMeta(pubkey);

    expect(meta.pubkey.equals(pubkey)).toBe(true);
    expect(meta.isSigner).toBe(false);
    expect(meta.isWritable).toBe(true);
  });

  it('should create readonly meta correctly', () => {
    const pubkey = Keypair.generate().publicKey;
    const meta = readonlyMeta(pubkey);

    expect(meta.pubkey.equals(pubkey)).toBe(true);
    expect(meta.isSigner).toBe(false);
    expect(meta.isWritable).toBe(false);
  });
});

describe('PDA Derivation', () => {
  it('should find PDA correctly', () => {
    const programId = Keypair.generate().publicKey;
    const seed = Buffer.from('my_seed');

    const [pda, bump] = findPda([seed], programId);

    expect(pda).toBeInstanceOf(PublicKey);
    expect(bump).toBeGreaterThanOrEqual(0);
    expect(bump).toBeLessThanOrEqual(255);

    // PDA should be deterministic
    const [pda2, bump2] = findPda([seed], programId);
    expect(pda.equals(pda2)).toBe(true);
    expect(bump).toBe(bump2);
  });

  it('should find PDA with multiple seeds', () => {
    const programId = Keypair.generate().publicKey;
    const seed1 = Buffer.from('prefix');
    const seed2 = Buffer.from('suffix');

    const [pda1] = findPda([seed1, seed2], programId);
    const [pda2] = findPda([seed2, seed1], programId);

    // Different seed order should produce different PDA
    expect(pda1.equals(pda2)).toBe(false);
  });
});

describe('Rent Calculation', () => {
  it('should calculate rent-exempt lamports', () => {
    const lamports = rentExemptLamports(100);
    expect(lamports).toBeGreaterThan(0);
    expect(typeof lamports).toBe('number');
  });

  it('should increase rent with data size', () => {
    const small = rentExemptLamports(10);
    const large = rentExemptLamports(1000);
    expect(large).toBeGreaterThan(small);
  });
});

describe('Slot Warping', () => {
  it('should warp to a future slot', () => {
    const svm = createLiteSvm();
    const payer = fundedKeypair(svm, 10 * LAMPORTS_PER_SOL);

    // Warp forward
    warpToSlot(svm, 100n);

    // SVM should still be functional after warping
    const blockhash = svm.latestBlockhash();
    expect(blockhash).toBeDefined();
  });
});

describe('Constants', () => {
  it('should have correct CALCULATOR_SIZE', () => {
    // discriminator (8) + owner (32) + display (8)
    expect(CALCULATOR_SIZE).toBe(48);
  });

  it('should have correct COUNTER_SIZE', () => {
    // discriminator (8) + count (8)
    expect(COUNTER_SIZE).toBe(16);
  });
});

describe('System Program Integration', () => {
  it('should execute a SOL transfer', () => {
    const svm = createLiteSvm();
    const sender = fundedKeypair(svm, 10 * LAMPORTS_PER_SOL);
    const receiver = Keypair.generate();

    // Create transfer instruction
    const transferIx = SystemProgram.transfer({
      fromPubkey: sender.publicKey,
      toPubkey: receiver.publicKey,
      lamports: LAMPORTS_PER_SOL,
    });

    // Execute transfer
    const result = executeTx(svm, transferIx, sender, [sender]);

    // Result should indicate success
    expect(result).toBeDefined();

    // Check receiver balance
    const receiverAccount = getAccount(svm, receiver.publicKey);
    expect(Number(receiverAccount?.lamports ?? 0)).toBe(LAMPORTS_PER_SOL);
  });
});
