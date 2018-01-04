# Changelog

## Unreleased (0.9.0)

* Replace ad-hoc API with a flexible `Transport` trait that can be implemented to change the way the request is sent
* Make the `reqwest` dependency optional - you can opt out and define your own `Transport` instead
* Add `Request::call_url`, an easy to use helper that calls a `&str` URL without needing to depend on `reqwest` in downstream crates
* Stricter checking of server headers

## <= 0.8.0

* The API slowly grew to expose more internals in order to accommodate more use cases
