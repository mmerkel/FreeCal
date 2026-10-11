// The types of the Rust core's application interface, generated from the
// Rust ones (see core/src/typescript.rs).
export type * from './types.generated';

/**
 * `T` with plain numbers in place of its branded IDs, as serialized samples
 * and JSON fixtures have them.
 */
export type Unbranded<T> = T extends number
  ? number
  : T extends string | boolean
    ? T
    : T extends readonly (infer Item)[]
      ? Unbranded<Item>[]
      : { [Key in keyof T]: Unbranded<T[Key]> };
