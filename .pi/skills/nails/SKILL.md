---
name: nails
description: A nail is an issue in the `nail` issue tracker. Use this when you fail to use the CLI for issue tracking, the user provided a nail key like nl-abc, or you want to change status, set up parent/child/blocking relationships (MANDATORY for related work), or run into other nail problems
---

# Nail CLI Usage

Nails are issues (bugs or tasks) tracked as markdown files in docs/nails/ in pwd or a parent dir. Each nail has a unique key like `nl-abc` with project-specific prefix.

## MANDATORY Rules of Nails

* Nails are issues tracking tasks or bugs so usual issue tracking best practices apply.
* A nail MUST include all relevant context to allow someone to start work on it.
* Reference context already in parent nails where possible instead of duplicating it.
* Relationships MUST be used when relevant as per below

## Creating Nails

```sh
# Create a task with priority severe
nail new task p1 'Title of task' 'Multiline markdown description
of what needs to be done referencing relevant files with level
of detail appropriate to complexity of task'

# New bug with priority moderate
nail new bug p2 'Title of bug' 'Multiline markdown description of
what is broken using standard repro, observed, expected, notes format,
leaving off irrelevant parts. Ok to vary format if format does not fit.'

# Create as child of existing nail
nail new --parent nl-xyz task 'Child task title' 'Description'
```

Priorities: p0 critical through to p4 trivial, p2 default

## Viewing Nails

All listing/search commands print a filter summary line before output (e.g. `Showing nails with status=open,wip`).

```sh
# Show work in progress and/or unblocked nails ready to start
nail pickup
nail pickup --type bug --priority p0,p1

# Show pickup restricted to a specific nail and its related nails
nail pickup nl-abc
nail pickup nl-abc nl-def --type bug

# List nails (defaults to open and wip status only)
nail list

# List ALL nails including closed
nail list --status open,wip,closed

# Filter by status
nail list --status open

# Filter by type and priority to show bugs at priority p0 or p1 (OR within a flag, AND between separate flags)
nail list --type bug --priority p0,p1

# Show full nail details
nail show nl-abc

# Search nail content (defaults to open,wip; supports usual -i, -C, -A, -B flags)
nail grep 'search term'
nail grep --regex 'rust regex pattern.*here' -i
nail grep 'search term' --status open,wip,closed --type bug
```

## Updating Nails

```sh
nail update nl-abc 'Multiline markdown description
goes here with relationships specified inline as per below.
Avoid using heredocs'
```

Can also include frontmatter to update frontmatter fields like priority or parent child relationships.

Can update with just frontmatter, just description, or both.

## Status Transitions

```sh
# Start working on a nail
nail wip nl-abc

# Put back to open
nail open nl-abc

# Close with resolution
nail close nl-abc done 'Implemented the feature as designed'
nail close nl-abc 'wont_fix: Out of scope for this release'
```

Resolution types: `done`, `wont_fix`, `cannot_repro`, `duplicate`, `invalid`

## Relationships

### Parent/Child
Set in frontmatter with `parent = "nl-xyz"`. Parents cannot be closed while any descendant is open/wip. Children cannot be opened/wip'd if parent is closed.

MUST use child relationships to organize strongly related nails into a hierarchy, especially using child nails to ensure procedures and required steps are followed.

### Blocking
In description: `blocked_by:#nl-xyz` or `blocks:#nl-xyz`. A blocked nail cannot be closed until the blocking nail is closed.

MUST use blocking relationships when one nail blocks another or work must be done in specific order.

### Mentions
In description: `#nl-xyz` or `mentions:#nl-xyz`

RECOMMENDED to use mentions to link related nails together when parent/child hierarchical relationships are not appropriate.

## Viewing Relationships

```sh
# all relationships
nail related nl-abc
# specific relationships
nail related nl-abc blocked_by
nail related nl-abc child
# etc
```

## Managing Relationships

```sh
# Set parent
nail reparent nl-abc nl-xyz

# Remove parent
nail unparent nl-abc

# Update content to modify other relationships (specify relationships in description or parent in frontmatter)
nail update nl-abc 'New description with blocks:#nl-xyz'
```

## Maintenance

```sh
# Check for integrity issues
nail doctor

# Fix auto-fixable issues
nail doctor --fix
```

## Nail Data Format

Prefer using `nail update nl-abc` with multiline strings for updates over editing files directly.

```md
+++
title = "Arbitrary title string, where filesystem name will be updated to match this title when using nail update"
type = "bug"
priority = "p2"
created_at = "2026-01-30T00:57:56Z"
parent = "nl-z52"
+++

Relevant *markdown* description goes here, with exact format and level of detail as per issue tracking best practices.

## Relationships demo

This issue blocks:#nl-12 and is itself blocked_by:#nl-a7 and we can mention #nl-24 which will all show up in related nails for either nail (as will parent/child relationships).
```

That nail would be stored in `docs/nails/open/` with filename `nl-1kl-bug-arbitrary-title-string-where-filesystem-name-will-.md` for open nails, or in `wip/` or `closed/` for those statuses. However MUST use `nail open` / `nail wip` / `nail close` to transition nail statuses to ensure database integrity is maintained.
