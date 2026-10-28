# ADR-0004: Verify kernel write denial before source SQLite access

## Status
Accepted, 2026-10-28, following independent safety review.

## Context
An outside-write canary returning PermissionDenied is useful but can also result
from ordinary ACLs. The fixed internal worker must reject an unsandboxed manual
invocation before SQLite can update WAL shared-memory read marks.

## Decision
Query the current process sandbox for write-data/create/unlink on the existing
canonical source directory and DB/WAL/SHM files before opening SQLite. Require
explicit kernel denial, not an error or an ACL-based failed write. Retain the
separate owned canary and normal fixed-profile launch. Missing main DB is rejected.

Use macOS sandbox_check, whose C declaration and filter constants are maintained
in WebKit's Darwin SandboxSPI.h. This is an OS-specific SPI: any query failure
fails closed; other platforms remain unsupported. One small module contains the
C ABI bridge and immutable flag read, with bounded CString lifetimes and explicit
safety comments. No source file is opened for writing or modified by this query.

The workspace retains unsafe_code=forbid. Only provider-api changes its inherited
lint to deny, with a single documented module allowance; architecture checks
reject unsafe code or allowances anywhere else. No new native crate or lifecycle
interface is introduced. This narrow exception is safer than treating one
filesystem canary as proof of the worker's sandbox policy.

## Validation
Tests distinguish an ordinary permission-denied file from actual MAC denial;
existing owned-directory canaries prove destination write access, source metadata
read access and source content denial. Live capture must pass this gate before
source SQLite is accessed. Regression tests also ensure leading-NUL content is
removed from every redacted field and raw persisted pages.

Primary declaration: https://github.com/WebKit/WebKit/blob/main/Source/WTF/wtf/spi/darwin/SandboxSPI.h
