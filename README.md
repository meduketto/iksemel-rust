# iks

Copyright (c) 2000-2025 Gurer Ozen <meduketto at gmail.com>

[iks][iks] is an XML parser library for [Jabber/XMPP][XMPP] and
general XML processing applications. It aims to be easy to use,
fast, and usable in resource-constrained environments.

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

# Usage

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
