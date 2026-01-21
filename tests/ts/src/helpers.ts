/**
 * LiteSVM-specific test helpers for Seahorse programs
 *
 * These helpers make it easy to set up LiteSVM integration tests for
 * Seahorse-compiled Anchor programs using TypeScript.
 */

import { LiteSVM } from 'litesvm';
import { Keypair, PublicKey, LAMPORTS_PER_SOL, Transaction, TransactionInstruction, AccountMeta } from '@solana/web3.js';
import { createHash } from 'crypto';

/** Path to compiled Seahorse program binaries */
export const PROGRAM_DEPLOY_DIR = '../../target/deploy';

/**
 * Create a LiteSVM instance and load a program from .so file
 *
 * @param programId - The program's public key
 * @param programPath - Path to the .so file
 * @returns A configured LiteSVM instance with the program loaded
 */
export function liteSvmWithProgram(programId: PublicKey, programPath: string): LiteSVM {
  const svm = new LiteSVM();
  // LiteSVM JS binding loads programs from file path via addProgramFromFile
  svm.addProgramFromFile(programId, programPath);
  return svm;
}

/**
 * Create a new LiteSVM instance
 *
 * @returns A fresh LiteSVM instance
 */
export function createLiteSvm(): LiteSVM {
  return new LiteSVM();
}

/**
 * Create a funded keypair in the SVM
 *
 * @param svm - The LiteSVM instance
 * @param lamports - Amount of lamports to fund
 * @returns A keypair with the specified lamports
 */
export function fundedKeypair(svm: LiteSVM, lamports: number = LAMPORTS_PER_SOL): Keypair {
  const keypair = Keypair.generate();
  svm.airdrop(keypair.publicKey, BigInt(lamports));
  return keypair;
}

/**
 * Create a funded keypair with 10 SOL
 *
 * @param svm - The LiteSVM instance
 * @returns A keypair with 10 SOL
 */
export function fundedKeypair10Sol(svm: LiteSVM): Keypair {
  return fundedKeypair(svm, 10 * LAMPORTS_PER_SOL);
}

/**
 * Calculate the Anchor instruction discriminator
 *
 * Uses the format: sha256("global:<instruction_name>")[..8]
 *
 * @param instructionName - Name of the instruction (snake_case)
 * @returns 8-byte discriminator as Buffer
 */
export function instructionDiscriminator(instructionName: string): Buffer {
  const hash = createHash('sha256');
  hash.update(`global:${instructionName}`);
  return hash.digest().subarray(0, 8);
}

/**
 * Calculate the Anchor account discriminator
 *
 * Uses the format: sha256("account:<AccountName>")[..8]
 *
 * @param accountName - Name of the account (PascalCase)
 * @returns 8-byte discriminator as Buffer
 */
export function accountDiscriminator(accountName: string): Buffer {
  const hash = createHash('sha256');
  hash.update(`account:${accountName}`);
  return hash.digest().subarray(0, 8);
}

/**
 * Create an Anchor instruction with the given discriminator and data
 *
 * @param programId - The program to call
 * @param instructionName - Name of the instruction (snake_case)
 * @param data - Serialized instruction arguments (without discriminator)
 * @param accounts - Account metas for the instruction
 * @returns TransactionInstruction ready to be executed
 */
export function anchorInstruction(
  programId: PublicKey,
  instructionName: string,
  data: Buffer | Uint8Array,
  accounts: AccountMeta[]
): TransactionInstruction {
  const discriminator = instructionDiscriminator(instructionName);
  const instructionData = Buffer.concat([discriminator, Buffer.from(data)]);

  return new TransactionInstruction({
    programId,
    keys: accounts,
    data: instructionData,
  });
}

/**
 * Create account meta for a writable signer
 */
export function signerMeta(pubkey: PublicKey): AccountMeta {
  return { pubkey, isSigner: true, isWritable: true };
}

/**
 * Create account meta for a writable non-signer
 */
export function writableMeta(pubkey: PublicKey): AccountMeta {
  return { pubkey, isSigner: false, isWritable: true };
}

/**
 * Create account meta for a read-only non-signer
 */
export function readonlyMeta(pubkey: PublicKey): AccountMeta {
  return { pubkey, isSigner: false, isWritable: false };
}

/**
 * Execute a transaction with the given instruction and signers
 *
 * @param svm - The LiteSVM instance
 * @param instruction - The instruction to execute
 * @param payer - The transaction fee payer
 * @param signers - All signers for the transaction
 * @returns Transaction result
 */
export function executeTx(
  svm: LiteSVM,
  instruction: TransactionInstruction,
  payer: Keypair,
  signers: Keypair[]
): any {
  const blockhash = svm.latestBlockhash();
  const tx = new Transaction();
  tx.recentBlockhash = blockhash;
  tx.feePayer = payer.publicKey;
  tx.add(instruction);
  tx.sign(...signers);

  return svm.sendTransaction(tx);
}

/**
 * Execute multiple instructions in a single transaction
 *
 * @param svm - The LiteSVM instance
 * @param instructions - The instructions to execute
 * @param payer - The transaction fee payer
 * @param signers - All signers for the transaction
 * @returns Transaction result
 */
export function executeTxMulti(
  svm: LiteSVM,
  instructions: TransactionInstruction[],
  payer: Keypair,
  signers: Keypair[]
): any {
  const blockhash = svm.latestBlockhash();
  const tx = new Transaction();
  tx.recentBlockhash = blockhash;
  tx.feePayer = payer.publicKey;
  instructions.forEach((ix) => tx.add(ix));
  tx.sign(...signers);

  return svm.sendTransaction(tx);
}

/**
 * Get an account from the SVM
 *
 * @param svm - The LiteSVM instance
 * @param pubkey - The account's public key
 * @returns Account info or null if not found
 */
export function getAccount(svm: LiteSVM, pubkey: PublicKey): any | null {
  return svm.getAccount(pubkey);
}

/**
 * Calculate rent-exempt lamports for a given data size
 *
 * @param dataSize - Size of account data in bytes
 * @returns Minimum lamports for rent exemption
 */
export function rentExemptLamports(dataSize: number): number {
  // Standard rent calculation: 2 years of rent
  // Base rent: 3480 + 2.4 lamports per byte per year
  const RENT_EXEMPT_MULTIPLIER = 2;
  const BASE_RENT_LAMPORTS_PER_YEAR = 3480;
  const RENT_LAMPORTS_PER_BYTE_PER_YEAR = 2.4;

  return Math.ceil(
    (BASE_RENT_LAMPORTS_PER_YEAR + dataSize * RENT_LAMPORTS_PER_BYTE_PER_YEAR) *
      RENT_EXEMPT_MULTIPLIER
  );
}

/**
 * Warp the SVM clock forward by the given number of slots
 *
 * Useful for testing time-dependent behavior like vesting or auctions
 *
 * @param svm - The LiteSVM instance
 * @param slot - Target slot number
 */
export function warpToSlot(svm: LiteSVM, slot: number | bigint): void {
  svm.warpToSlot(BigInt(slot));
}

/**
 * Find a PDA with bump seed
 *
 * @param seeds - Array of seeds (Buffers or Uint8Arrays)
 * @param programId - The program that owns the PDA
 * @returns [address, bump]
 */
export function findPda(
  seeds: (Buffer | Uint8Array)[],
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(seeds, programId);
}

/** Standard account size for Seahorse Calculator account */
export const CALCULATOR_SIZE = 8 + 32 + 8; // discriminator + owner + display

/** Standard account size for a simple counter account */
export const COUNTER_SIZE = 8 + 8; // discriminator + count
