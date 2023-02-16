# addr-spec

[![crates.io](https://img.shields.io/crates/v/addr-spec?style=flat-square)](https://crates.io/crates/addr-spec)
[![license](https://img.shields.io/crates/l/addr-spec?style=flat-square)](https://github.com/mathematic-inc/addr-spec-rs)
[![ci](https://img.shields.io/github/actions/workflow/status/mathematic-inc/addr-spec-rs/ci.yaml?label=ci&style=flat-square)](https://github.com/mathematic-inc/addr-spec-rs/actions/workflows/ci.yaml)
[![docs](https://img.shields.io/github/actions/workflow/status/mathematic-inc/addr-spec-rs/docs.yaml?label=docs&style=flat-square)](https://github.com/mathematic-inc/addr-spec-rs/actions/workflows/docs.yaml)

A wicked fast UTF-8 email address parser and serializer. It provides

- unopinionated, _correct_ parsing of email addresses (defined as `addr-spec` in
  [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322)),
- extremely fast serialization and deserialization using low-level memory
  management,
- guarantees on the uniqueness of an email address,
- UTF-8 support with NFC normalization as recommended in [RFC
  6532](https://datatracker.ietf.org/doc/html/rfc6532), and
- format validation based on the grammar set out in [Section 3.4.1, RFC
  5322](https://www.rfc-editor.org/rfc/rfc5322#section-3.4.1) and [Section 3.2,
  RFC 6532](https://datatracker.ietf.org/doc/html/rfc6532#section-3.2) with
  position-accurate errors.

## Features

This crate supports the following features:

- `normalization` - This enables (NFC) normalization of addresses.
- `comments` - This allows parsing (but not serialization; see
  [Caveats](#comments)) of comments.
- `literals` - This allows parsing and serialization of literal domains.
- `white-spaces` - This allows parsing (but not serialization; see
  [Caveats](#folding-white-spaces)) of whitepaces.

By default, `normalization` is enabled.

## Caveats

### Folding white spaces

Serializing folding white spaces (abbr. FWS) is not supported since it is
dependent on the transport mechanism. If you need to break the address along
foldable boundaries, you can use `into_serialized_parts` which returns
serialized versions of the local part and domain.

### Comments

Serializing comments is not supported since it is dependent on the transport
mechanism. At the moment, comments are parsed, but skipped as there is no
uniform way of handling them. If you would like comments to be stored, please
file an issue with your use-case.

## Alternatives

### [email_address](https://docs.rs/email_address/latest/email_address/)

This crate provides a "newtype" `EmailAddress` which under the hood just
validates and wraps an address string.

It does not support white spaces, comments, and UTF-8 normalization, nor does it
support address normalization (e.g. `"te st"@example.com` is equivalent to
`test@example.com`, but this cannot be distinguished with `email_address`).

#### Benchmarks

In scenarios supported by `email_address` (no comments, no white-spaces, no
UTF-8/address normalization), `email_address` slightly outperforms `addr_spec`
by about 5% with all features off which makes sense since `email_address` cannot
distinguish equivalent addresses.

#### Migration

It's highly recommended to use only `addr_spec` in production since `addr_spec`
provides guarantees on uniqueness for storage and lookup as well as other
special perks (position-based errors, SMTP-style `Display` writer, etc.). If
this is not feasible, we provide `Into<EmailAddress>` and `Into<AddrSpec>` for
those coming from `email_address`. Note that `Into<AddrSpec>` is a only a [right
inverse](https://en.wikipedia.org/wiki/Inverse_function#Left_and_right_inverses)
of `Into<EmailAddress>`, i.e. `AddrSpec -> EmailAddress -> AddrSpec` will always
yield the same `AddrSpec`, but `EmailAddress -> AddrSpec -> EmailAddress` may
not yield the same `EmailAddress`.
 