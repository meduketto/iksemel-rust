# 0.3.0 (2025-10-24)

* New SyncCursor allows thread-safe multiple cursors to the same
  document with reference counting. Useful for long-living
  separately stored cursors into same XML tree.
* Fixed a bug where the SaxParser was returning &str references
  to incomplete UTF8 sequences at the end. Parser now buffers the
  last sequence (4 bytes max) and sends it as a whole.

# 0.2.0 (2025-10-08)

## Breaking changes

* `SaxElement::EmptyElementTag` is replaced with `StartTagEmpty`, and
  a new `StartTagContent` element is added. Now you get one of these
  after receiving any attributes to indicate that the start tag
  is fully processed. This was necessary for `StreamParser` to detect
  the stream start tag.
* `SaxParser` now returns each `SaxElement` as it is parsed via new
  `SaxElements` lending iterator instead of the clumsy handler trait.
  Same pattern is used in Document and Stream parsers as well.
* Since the handler callback is not used anymore, `SaxError` and
  `DocumentError` which ended up with exact same error variants are
  consolidated into the `ParseError` object.

## New features

* Cursor now provides `following_sibling`, `preceding_sibling`,
  `ancestor` iterators.
* New `StreamParser` produces Documents for each XMPP stream top level.
* New sans-io `XmppClientProtocol` protocol handler.
* New `XmppClient` blocking-io client library.
* iksjab cmdline tool to send messages or backup roster (replacement
  for the old iksroster tool).
* CI and release processes automated with GH actions.

# 0.1.0 (2025-09-06)

First release of the Rust port of iksemel.

The crate name is specified as 'iks', since that corresponds
to the original C library prefix of 'iks_*' and provides a
familiar namespace.

Apart from the rigorous testing and type safe APIs provided
by Rust, this release also brings improvements over the
original C implementation:

* Character references in the attribute values are properly parsed.
* DTDs are correctly skipped, but still not used.
* Valid character checks are now more strict according to the XML
  specification. Longer than four bytes UTF8 sequences are rejected.
* Experimental XPath support with new 'ikspath' command line tool.

The Python bindings and the full XMPP code are not ported in this
release, but coming soon.
