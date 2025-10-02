# Design Choices of Iksemel

## Goals

Iksemel is designed for three types of applications:

1. Simple XML processors.

2. Document building and querying applications which keep the data
   as XML tree in the memory.

3. Network protocol applications which use XML as the line or message
   format, such as XMPP. These can be clients or servers.

There are four main engineering goals:

1. Ease of use. It should be easy to write the applications. API should
   be elegant, flexible, safe to use, and intuitive.

2. Performance. This mainly depends on cache usage on modern processors,
   therefore more compact data representations should be used. Slow,
   indeterministic operations like memory allocations should be reduced
   to a minimum as well.

3. Supporting resource limited targets. Using minimal system resources
   and operating system facilities is helpful for a wide range of
   environments from containers to embedded systems.

4. Engineering quality. Rigorous testing, well designed APIs, not
   panicking under low memory conditions, having good documentation
   are all important.

## Non Goals

### Serializing

Iksemel is explicitly not a serializer like `serde`. Parsing the data
directly into a native data structure could be more efficient but also
loses all the extensibility, the 'X' in the 'XML'. Iksemel operates
entirely in the XML data model, and the users can introduce their
abstractions if they choose.

### Full Compliance With XML Specifications

The XML format was created as a response to the complexity of older
standards like SGML, but came up with its own problematic constructs,
especially in some of the other standards built on top of it. Other
decisions which seemed good at the time did not age well.

Thankfully the XMPP spec outright forbids such things like the
processing instructions or DTDs, and people moved away from other
problematic things as they realize the problems. Modern formats
like JSON, TOML and YAML also affected the best practices.

Thereforce, Iksemel does not aim to support every feature, and
rejects supporting the following subset:

### Non UTF-8 Character Sets

UTF-16 was created as a simple to decode alternative to UTF-8, but
still suffered from complexity because of endianness issues, and
still requiring complex decoding for the full Unicode set.

UTF-32 can encode the entire Unicode set with no multi-byte
decoding but incredibly wasteful in terms of memory and CPU.

Furthermore, people realized that even for non Latin languages
which constantly need multi-byte sequences, UTF-8 still results
in smallest memory use due to most characters still being spaces,
numbers, punctuation, and other similar single byte encodings.

Iksemel only supports UTF-8, but it is still very easy to convert
other sets before sending them to the parser if needed.

### DTD

This was a badly limited validation system which is obsoleted
within a few years by RelaxNG, and then others like XSD,
Schematron, etc. It uses an ad-hoc, non-XML language which is
embedded within a DOCTYPE element.

It was a source of many security vulnerabilities, such as the
[External Entity][XXEATTACK] or [Billion Laughs][BILLIONLOL] attacks.

Iksemel just skips it.

### Named Entities

These depend on being defined under the DTD, and therefore not
supported. This kind of text substitution should not be needed
in a markup language which can refer to shared data in million
other ways.

Rather than returning an unintended output, Iksemel will return
an error if it encounters them in a CData element.

### Processing Instructions

Why would anyone want a non-XML way to pass information when the
format itself is extensible and you can have all sorts of extra
elements under your own namespace?

Passing non-standard instructions to the processor to change its
behavior can also defeat the entire interoperability goal of XML.

Iksemel just skips them.

## Decisions

### Chunked Parsing

Since data is coming as chunks from network, and not loading the
entire file is quite a memory saving, all parser APIs accept a
variable length of bytes, and allow the caller to do a final
validation when the entire document has been fed.

This requires the DOM and Stream parsers to copy some data, but
the efficient Arena implementation and the smart packing of tree
structs makes this quite fast. This also allows combining of CData
sections escaped with various XML structures into a single
continous string.

### Comment Processing

Comments are skipped but not returned to the application. Since
they are intended for human editors' consumption, preserving them
through machine processing would not be useful. It is very easy
to include a namespaced comment/description as an actual XML
element if there is a need for machine processing.

### NoMemory Errors

It is really not good for a server handling ten thousand requests
to crash and take down all when one request tries to make a big
allocation.

It can be argued that, this is not going to help when the allocation
is tiny, as something else will inevitable fail anyway.

Whether small or not, Iksemel always checks allocations and returns
a NoMemory error when they fail.

### Null Cursors

It is possible to end up with nothing when navigating or querying
the document tree. Rather than using an `Option` to represent this
state, the Cursor changes its internal pointer to a null value, and
subsequent operations do NOT do anything.

This is not very idiomatic in Rust, but it allows a very elegant
chaining of multiple navigation operations without putting them
into individual functions to be able to use the `?` operator, or
writing bunch of extra `.or(...)` and `.and_then(...)`s etc.

This is a remnant of how it was working in the C version, and I am
not very happy about it. Once the `try` block is stabilized in the
Rust language, I will switch to it.

Note that the cursor editing functions have a idiomatic `Result` return
type as their failures almost always require short cutting the chain,
and then exiting from the containing function.

### XPath

XPath is a kitchen sink language, trying to pack too much into a
simple query string. Its data model has incompatible changes
across the versions. Despite all the shortcomings, its basic
filtering syntax is workable, and better than inventing a new one.

Iksemel Document API tries to make XPath axis traversals and various
predicate filterings easy to program. That way, we can implement
similar queries in a better language.

But a string query is sometimes useful too. That is why the `ikspath`
tool is provided. It implements a basic subset of XPath syntax.
Underlying implementation of XPath parsing and execution is shipped
as a part of the library crate, but not necessarily at the same
quality level or complies with the similar standards as the rest of
the library. This might change in the future, but it is unlikely
that any complicated features of XPath will be supported.


[BILLIONLOL]: https://en.wikipedia.org/wiki/Billion_laughs_attack
[XXEATTACK]: https://en.wikipedia.org/wiki/XML_external_entity_attack
