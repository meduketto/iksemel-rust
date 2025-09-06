# Contributing to iks

Thank you very much for your interest in contributing! :heart:

## Ways to Contribute

There are many ways you can contribute to iks. You can:

### 1. Share your experience

I'm interested in hearing about your experience with iks whether
it is negative or positive. Feel free to write me an email, or
publish a blog post!

### 2. Submit a bug report

Bug reports are always welcome. Most useful reports include a
preferably minimal way to reproduce the issue, such as an example
XML document or a code snippet. If you also have trouble reliably
reproducing the issue, a report with as much as information you
can collect is still useful. I will try my best to fix bugs
within a reasonable timeframe.

You can report via GitHub issues, or directly to my email address
if the information is sensitive.

### 3. Send a feature request

Feature requests are welcome too, but please check if they align
with the project goals and philosophy described in the DESIGN.md
file.

Be aware that this is a spare time project for me, and I have
real work and life, so feature requests are not guaranteed to be
implemented. Also see the next way below.

Always use GitHub issues for feature requests.

### 4. Write code, tests, documentation


#### Licencing

#### Coding Standards

* All code must be formatted with `cargo fmt`.
* Code must be passing `cargo clippy`. If there is a good reason to
  ignore a rule, it must be done in the smallest possible scope, and
  a reason must be specified in the `#[allow()]` attribute.
* Code must be passing `cargo test` under `miri`. Since miri is
  slow, I suggest using it only before the final submission.
* Bugfixes and new features must be accompanied by unit tests.
* Unit tests must be tested for decent coverage with `cargo mutants`.
  Not all misses need to be caught by tests, but they should be
  reviewed.
* Public API must have `rustdoc` comments with examples.
* Dependencies can only be added if they are absolutely necessary,
  not for convenience.
* Backwards incompatible API changes must be avoided, unless they
  provide significant type safety or performance improvements.
