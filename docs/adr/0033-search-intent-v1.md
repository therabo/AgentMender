# ADR-0033 — SearchIntent v1

SearchIntent v1 is the ordinary-language input to the caller-scoped resolver
from [ADR-0032](0032-caller-scoped-session-resolution.md). A project mention
describes the subject until the user explicitly restricts project membership.
The originating chat checks Luna's interpretation against the live request.
The installed MCP route supplies apparent caller metadata and does not
authenticate the original user wording.

The pure domain compiler validates typed constraints and compiles them to
the existing three-valued query. The lexical branch generates provisional
candidates. Project hints travel with the intent for semantic assessment.
The original hard constraints stay with the run across a bounded lexical
expansion. Advanced exact Boolean requests can use the existing structured
resolver or discovery contract.

The same confined reader, transient FTS5/BM25, source revisions, run store
and evidence publication checks serve both resolver inputs. An exhaustive
semantic claim requires eligible semantic coverage beyond lexical acquisition.
The resolver exposes incomplete coverage and still publishes fresh supported
matches. The originating chat owns all capabilities and uses Luna Medium
for the proposed intent and evidence judgments where the host supports it.

P0-SR5.2 keeps the same run and trust boundaries. The ordinary host workflow
assesses the first useful batch and finishes once there is a supported
standard answer. A single delegate handles interpretation and judgments
when the origin needs one. Exact title and inventory questions use metadata.
The worker prepares ranking terms per inventory pass and constructs transient
FTS only for slices with evidence to rank. When ranking is needed, BM25
still uses the full slice. See the [originating chat workflow](../intent-aware-resolution.md).
