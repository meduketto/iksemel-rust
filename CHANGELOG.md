# 0.2.0 (TBD)

* Cursor now provides following_sibling, preceding_sibling
  iterators.

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
