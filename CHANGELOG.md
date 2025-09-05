# 0.1.0 (unreleased)

First release of the Rust port of iksemel.

Apart from the rigorous testing and type safe APIs provided
by Rust, this release also brings improvements over the
original C implementation:

* Character references in the attribute values are properly parsed.
* DTDs are correctly skipped, but still not used.
* Valid character checks are now more strict according to the XML
  specification. Longer than four bytes UTF8 sequences are rejected.
* Experimental XPath support with new 'ikspath' command line tool.
