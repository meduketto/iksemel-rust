# iksemel

Copyright (c) 2000-2025 Gurer Ozen <meduketto at gmail.com>

[iks][iks] is an XML parser library for [Jabber/XMPP][XMPP] and
general XML processing applications. It aims to be easy to use,
fast, and usable in resource-constrained environments.

![Crates.io Version](https://img.shields.io/crates/v/iks)![GitHub License](https://img.shields.io/github/license/meduketto/iksemel-rust)


# Features

* SAX API for minimal memory usage.
* DOM API for flexible and powerful document querying and editing.
* Stream API for efficient XMPP stream processing.
* Experimental basic XPath support.

# Non-features

* Only accepts UTF-8 encoded XML.
* Document Type Definitions (DTD, the DOCTYPE element basically) are
  syntactically parsed but not used for validation or custom entity
  definition.
* Processing instructions and comments are parsed but not passed to
  the application.

These are all intentional decisions rather than technical limitations,
and will never be implemented. See DESIGN.md file for the rationale
and alternative solutions.

# Installation

You can use it in your rust projects by adding the following to your `Cargo.toml`:

```toml
[dependencies]
iks = "0.1.0"
```

You can also install it via cargo:

```sh
cargo install iks
```

which also installs the command line tools.

# Usage

See API documentation for detailed examples and information.

Here is a simple example:

```rust
use iks::Document;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = Document::from_str("<doc><a>123</a><b><a>456</a><a>789</a></b></doc>")?;

    let count = doc
        .root()
        .descendant_or_self()
        .filter(|element| element.name() == "a")
        .enumerate()
        .map(|(index, element)| element.set_attribute("nr", Some(&index.to_string())))
        .count();

    assert!(count == 3);
    assert!(
        doc.to_string()
            == "<doc><a nr=\"0\">123</a><b><a nr=\"1\">456</a><a nr=\"2\">789</a></b></doc>"
    );

    Ok(())
}
```

# Tools

Iksemel provides a few command line tools for quick XML processing:

* ikslint: validates XML files
* ikspath: loads XML files into a DOM tree and runs XPath queries on them

# Contributing

There are many [ways to contribute](CONTRIBUTING.md) to the project. See also
the [design decisions](DESIGN.md) document.

# License

Iksemel is free software: you can redistribute it and/or modify it under
the terms of the GNU Lesser General Public License as published by the
Free Software Foundation, either version 3 of the License, or (at your
option) any later version.

Iksemel is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public
License for more details.

You should have received a copy of the GNU Lesser General Public License
along with Iksemel. If not, see <https://www.gnu.org/licenses/>.


[iks]: https://github.com/meduketto/iksemel-rust
[XMPP]: https://xmpp.org
