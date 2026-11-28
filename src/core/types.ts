// The types of the Rust core's application interface, generated from the
// Rust ones (see core/src/typescript.rs).
export type * from './types.generated';

/**
 * `T` with plain numbers and strings in place of its branded IDs and keys,
 * as serialized samples and JSON fixtures have them.
 */
export type Unbranded<T> = T extends number
  ? number
  : T extends string & { readonly __brand: string }
    ? string
    : T extends string | boolean
      ? T
      : T extends readonly (infer Item)[]
        ? Unbranded<Item>[]
        : { [Key in keyof T]: Unbranded<T[Key]> };
