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
names. Its markers do not spell private repository names (that would
break this rule too), so review is what holds the rest of the tree to
it.
