# Contributing to iksemel

Thank you very much for your interest in contributing! :heart:

## Ways to Contribute

There are many ways you can contribute to iksemel. You can:

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

A Pull Request (PR) on the Github repository is the most preferred
form for such contributions.

#### License

By sending a PR, you agree to license your code under the same
terms as the iksemel project, which is LGPL3 or later.

For bug fixes and similar small changes, I will require you to
assign your copyright as well to the project by including the
following line in your commit message:

```
I hereby assign copyright in my code to the iksemel project,
to be licensed under the same terms as the rest of the code.
```

For large or substantial changes, you may retain your copyright
by adding it to the file header.

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
* Unstable language features must not be used.
* Modules must be layered and built on top of each others public
  interfaces.
* There are negative tests for the unsafe code which verifies that
  the API cannot be used incorrectly. Since there isn't a better
  native mechanism for these in Cargo, they are implemented as
  rustdoc tests with `compile_fail` attribute which only check for
  any compilation error. To make sure that compilation does not fail
  for unrelated reasons, it is necessary to check the output
  manually via `cargo test -- --no-capture` when changing the
  related code and tests.

#### Release Process

* Run `cargo mutants` to see if there is a need to add more unit tests.
* Check the version number in these places:
  - `Cargo.toml`
  - `CHANGELOG.md`
  - `iks.doap`
* Check the release notes in `CHANGELOG.md`:
  - Breaking changes must be documented with instructions for migration.
  - New features must be briefly mentioned.
* Run the tests locally to ensure that the build will not fail.
* Run the `Release` Github action. This action will run regular CI tests
  and Miri tests and will check the version numbers as well before
  publishing the release and tagging the source.
* Create a release record on the Github releases page.
* Update the version number for the next development cycle.
