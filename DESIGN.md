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

UTF-32 can encode the entire Unicode set which no multi-byte
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
Schematron, etc.

It uses an ad-hoc, non-XML language which is embedded within
a DOCTYPE element.

Iksemel just skips it.

### Named Entities

These depend on being defined under the DTD, and therefore not
supported. This kind of text substitution should not be needed
in a markup language which can refer to shared data in million
other ways.

Rather than returning an unintended output, Iksemel will abort
with an error if it encounters them in a CData element.

### Processing Instructions

Why would anyone want a non-XML way to pass information when the
format itself is extensible and you can have all sorts of extra
elements under your own namespace?

Passing non-standard instructions to the processor to change its
behavior can also defeat the entire interoperability goal of XML.

Iksemel just skips them.

## Decisions

### Chunked Parsing

### Comment Processing
