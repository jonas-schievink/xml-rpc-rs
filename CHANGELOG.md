# Changelog

## Unreleased

No changes.

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
