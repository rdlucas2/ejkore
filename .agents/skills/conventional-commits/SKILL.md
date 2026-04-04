---
name: conventional-commits
description: Enforce conventional commit message style for all git commits. Use when writing any git commit, creating commit messages, or when the user asks to commit changes. Always apply this skill before running git commit.
---

# Conventional Commits

All commits MUST follow the [Conventional Commits](https://www.conventionalcommits.org/) spec. No exceptions.

## Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

## Types

| Type | When to use |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, whitespace (no logic change) |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `chore` | Build process, dependency updates, tooling |
| `ci` | CI/CD configuration |
| `revert` | Reverts a previous commit |

## Rules

- **Subject line**: lowercase, no trailing period, max 72 chars
- **Scope**: optional, lowercase, in parentheses — e.g. `feat(auth):`
- **Breaking change**: append `!` after type/scope — e.g. `feat!:` or `feat(api)!:`
- **Body**: separated by blank line, wrap at 72 chars, explain *why* not *what*
- **Footer**: `BREAKING CHANGE: <description>` or issue refs like `Closes #123`

## Examples

```
feat(auth): add oauth2 login flow

fix: prevent race condition in job queue

docs: update README with setup instructions

chore(deps): bump typescript to 5.4

feat!: drop support for node 16

BREAKING CHANGE: minimum node version is now 18
```

## Validation Checklist

Before every commit:

- [ ] Type is one of the valid types above
- [ ] Subject is lowercase and under 72 chars
- [ ] No trailing period on subject line
- [ ] Breaking changes marked with `!` and/or `BREAKING CHANGE:` footer
- [ ] Body (if present) explains motivation, not mechanics
