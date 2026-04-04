---
name: code-review
description: Guide for conducting code reviews. Use when reviewing pull requests, auditing code quality, identifying security issues, or providing code feedback.
---

# Code Review Best Practices

## When to Use This Skill

Activate when:
- Reviewing pull requests
- Conducting code audits
- Providing feedback on code quality
- Identifying security vulnerabilities
- Suggesting refactoring improvements
- Checking adherence to coding standards

## Code Review Checklist

### 1. Correctness
- Logic is correct and handles all cases (nulls, empty lists, negatives, off-by-one)
- Edge cases and error conditions are considered
- Return values and assertions are correct

### 2. Security
- No SQL injection, XSS, CSRF vulnerabilities
- User input is validated and sanitized
- No hardcoded secrets or credentials
- Authentication and authorization are properly implemented
- Sensitive data is not logged
- File uploads validated (type, size, content)
- Rate limiting in place for APIs

### 3. Performance
- No N+1 query problems
- Appropriate data structures (e.g. set/map for lookups, not list)
- Database indexes are used
- Large datasets are paginated or streamed
- Resources are cleaned up properly

### 4. Code Quality & Maintainability
- Clear, descriptive variable and function names
- Functions are small and focused (single responsibility)
- No code duplication (DRY)
- Comments explain *why*, not *what*
- Magic numbers replaced with named constants
- Follows project conventions and style guide

### 5. Error Handling
- Errors don't silently fail (no empty catch blocks)
- Error messages are specific and helpful
- Errors are logged appropriately
- Both happy path and error paths are handled

### 6. Testing
- New functionality has tests
- Edge cases and error conditions are tested
- Tests are deterministic (no flaky tests)
- Test names describe what they test
- Coverage is adequate

### 7. Documentation
- Public APIs are documented
- Complex logic has explanatory comments
- README/changelog updated for user-facing changes

### 8. Dependencies
- New dependencies are justified
- Licenses are compatible
- No known security vulnerabilities
- Versions are pinned or bounded

## Feedback Labels

Use these to categorize every comment:

- **[blocking]** — Must be fixed before merging
- **[suggestion]** — Optional improvement
- **[question]** — Asking for clarification
- **[nit]** — Minor, cosmetic issue
- **[security]** — Security concern
- **[performance]** — Performance concern

## Providing Feedback

**Be specific and actionable:**

```markdown
# BAD
"This function is bad."

# GOOD
[blocking] This creates a SQL injection vulnerability. Use parameterized queries:
  # Instead of: query = "SELECT * FROM users WHERE name = '#{name}'"
  # Use: from(u in User, where: u.name == ^name)
```

- Explain the *why* behind suggestions
- Ask questions instead of making demands
- Praise good code
- Offer to pair on complex issues

## Review Process

1. **Read the PR description** — understand the problem being solved
2. **Big picture first** — is the approach sound? does it fit the architecture?
3. **Correctness** — does it work? edge cases handled?
4. **Security + performance** — any vulnerabilities or scale concerns?
5. **Code quality** — readable, maintainable, tested?
6. **Re-review after changes** — verify fixes are correct

## Self-Review Checklist (before submitting)

- [ ] Code compiles and all tests pass
- [ ] Tests added for new functionality
- [ ] No commented-out code or debug statements
- [ ] No secrets or sensitive data
- [ ] Commit messages are clear
- [ ] Changes are focused (no unrelated changes)
- [ ] Documentation updated

## Review Etiquette

**DO:** Be respectful, assume good intent, respond promptly, approve good code.

**DON'T:** Be sarcastic, bike-shed on style preferences, block on personal taste, approve code you don't understand.

## Common Code Smells

- **Long functions** — doing too much
- **Deep nesting** — extract to functions
- **Copy-paste code** — extract shared logic
- **Magic numbers** — use named constants
- **God object** — class/module doing everything
- **Unclear names** — `x`, `tmp`, `data`
