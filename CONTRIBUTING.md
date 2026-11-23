# Contributing to proved-port

## Nothing private in this repository

This repository is public, and the kit ships into ports, some of them
public. Several of the ports the method came from live in private
repositories. So nothing private goes into this repository: not in
docs, code comments, test data, commit messages, pull request bodies or
review replies. In particular, never write:

- the name of a private repository, or a name it had before;
- a link to a private repository, or to an issue, pull request, commit
  or file in one;
- an issue or pull request number, a commit sha, or a file path that
  comes from a private repository.

Refer to a private port by the library it replaces ("the PCRE2 port",
"the zlib port", "the Expat port"), not by its repository. When a kit
change comes from work in a private repository, describe the technique
in general terms only: what the check or tool does and why, never where
it was first written or which private issue asked for it.

`kit/internal-test.sh` scans the files the kit ships for internal
names, and every file git tracks for private repository names. It
does not spell those names (that would break this rule too): it holds
them as sha256 digests and reports a match by file, line and the
column of the word, never by name. The digests keep the names from
being written here; they are not a secret. A short name can be
recovered from its digest by trying candidates, and that trade-off is
accepted: the alternative, a private list, would leave the public
check blind.

If the check flags a word you meant in another sense, reword it; there
is no exemption. A private name that is also an everyday word or the
name of another public project has no digest, because it would flag
ordinary text; review holds those.

To add a name (for instance when a private repository is created whose
name lacks the `cg-` prefix the markers already catch; whoever creates
it, or the next review that meets it, adds it):

1. Write it in lower case with each `.` and `_` as `-`. The scan
   hashes only one to three parts joined by single `-`, so the name
   must have at most three parts, with no `--` and no leading or
   trailing separator; any other name needs the scan widened first, or
   its digest never matches and nothing reports that.
2. Add the output of `printf %s NAME | sha256sum` (no trailing newline:
   `printf %s`, not `echo`) to the script's `digests` list, in sorted
   order.
3. Check it outside the tree: copy `kit/` to a scratch directory, add a
   line holding the name to a shipped file there (for example
   `kit/make/headers.sh`), run the copy's `kit/internal-test.sh`, see it
   fail with that file and line, and delete the copy.
4. Give the name nowhere else: not in the commit message, the pull
   request or a review reply.

Review holds commit messages and pull requests to this rule.
