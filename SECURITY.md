# Security

Please report a vulnerability privately through **Security → Report a vulnerability** on this repository, not in a public issue.

Kensa only reads public pages: it sends GET requests, never logs in and never submits forms. It respects robots.txt for its own user agent (`kensa`), stays on the start origin and uses 4 parallel requests by default. `kensa serve` binds to 127.0.0.1 only and refuses paths outside the served folder.

The trap site and every name in it are fictional. The domain `lune-et-lin.example` is reserved and never resolves.
