# Changelog

## Unreleased

* Updated to Rust 2021 Syntax
* Updated `reqwest`to 0.11.10
* Added `tokio` dependency for async runtime (feature rt-multi-thread)

- Refactored to not use `reqwest::blocking`, all interfaces stay stable

## 0.15.1 - 2021-11-02

### New Features

- Added `From<Option<_>>` impls for `Value`, yielding either `Nil` or the contained value ([#76])

[#76]: https://github.com/jonas-schievink/xml-rpc-rs/pull/76

## 0.15.0 - 2021-01-23

### Breaking Changes

* Updated `iso8601` dependency to 0.4.0
* Updated `reqwest` dependency to 0.11.0

### Misc

* Changed request formatting to be more readable ([#68])
* Updated private dependencies

[#68]: https://github.com/jonas-schievink/xml-rpc-rs/pull/68

## 0.14.0 - 2020-02-06

### Breaking Changes

* Updated `iso8601` dependency to 0.3.0
* Updated `reqwest` dependency to 0.10.1
* Added a new default feature `tls` that can be disabled to turn off [reqwest]'s TLS support.

[reqwest]: https://github.com/seanmonstar/reqwest

## 0.13.1 - 2019-02-20

### Misc

* Update internal dependencies

## 0.13.0 - 2018-11-09

### Breaking Changes

* Update reqwest to 0.9 to fix openssl-related build failures
  ([#44](https://github.com/jonas-schievink/xml-rpc-rs/pull/44))

## 0.12.0 - 2018-08-24

### Breaking Changes

* Bump the minimum supported Rust version and change the Rust version policy.

  From now on, `xmlrpc` will adopt the same policy as [tokio] (on which we
  depend): We will support the current Rust version and the 2 releases prior to
  that (which currently means that we support 1.25.0+).

  Bumping the required Rust version is no longer considered a breaking change as
  long as the latest 3 versions are still supported.

[tokio]: https://github.com/tokio-rs/tokio

### New Features

* Add `Request::new_multicall` for easier execution of multiple calls via `system.multicall`

### Bugfixes

* Better handling of `Value::DateTime`
  * Print the timezone if the zone offset is non-zero
  * Print the fractional part of the time if it's non-zero
* Accept base64 values containing whitespace

## 0.11.1 - 2018-05-14

### Bugfixes

* Stop checking `Content-Length` headers to support compressed responses ([#41](https://github.com/jonas-schievink/xml-rpc-rs/pull/41))

## 0.11.0 - 2018-02-25

### Breaking Changes

* `Transport` errors must now be `Send + Sync`; this allows our own `Error` type to be `Send + Sync`, which makes it more useful for downstream crates (see: [API guidelines][c-good-err]) ([#39](https://github.com/jonas-schievink/xml-rpc-rs/pull/39))

## 0.10.0 - 2018-02-21

### Breaking Changes

* Replace ad-hoc API with a `Transport` trait that can be implemented to change the way the request is sent
* Stricter checking of server headers
* Removed the nested `Result` you get when performing a call
* Restructure the `RequestError` type to better hide details the user shouldn't need to see
* Rename `RequestError` to just `Error` to better match what other crates do
* Removed the `RequestResult` type alias in favor of explicitly naming the result type

### New Features

* Make the `reqwest` dependency optional - you can opt out and define your own `Transport` instead
* Add `Request::call_url`, an easy to use helper that calls a `&str` URL without needing to depend on `reqwest` in downstream crates
* Add the `http` module, containing a few helper methods for writing custom reqwest-based `Transport`s
* Derive a few more useful traits ([#34](https://github.com/jonas-schievink/xml-rpc-rs/pull/34))
* Implement `From<i64>` for `Value` ([#33](https://github.com/jonas-schievink/xml-rpc-rs/pull/33))
* Add methods `Value::get` and `Value::as_*`, implement `std::ops::Index` for `Value` for convenient access to wrapped
  data ([#37](https://github.com/jonas-schievink/xml-rpc-rs/pull/37)).

## <= 0.9.0

* The API slowly grew to expose more internals in order to accommodate more use cases

[c-good-err]: https://rust-lang-nursery.github.io/api-guidelines/interoperability.html#c-good-err
