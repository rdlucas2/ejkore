---
name: docker-best-practices
description: Write Dockerfiles following best practices: pinned image versions, efficient layer ordering, and a mandatory 3-stage multi-stage build (base, test, artifact). Use when creating or editing any Dockerfile, or when the user asks about Docker, containerization, or building images.
---

# Docker Best Practices

## Non-negotiables

- **Always pin image versions** — never use `latest` or unversioned tags
- **Always use multi-stage builds** — minimum 3 stages: `base`, `test`, `artifact`
- **Artifact stage runs as a non-root user**
- **Order layers for cache efficiency** — dependency files before source code

## The 3-Stage Pattern

### Stage 1 — `base`
Installs production dependencies and builds the application.

- Copy dependency manifest files **first** (e.g. `package.json`, `go.mod`, `requirements.txt`, `pom.xml`) and install deps before copying source — this caches the dependency layer
- Then copy source and compile/build
- Use the smallest appropriate builder image for the language

### Stage 2 — `test`
Extends `base`. Adds test tooling, runs tests, and generates a coverage report.

- `FROM base AS test`
- Install test runner and coverage tools
- Output results to `/out/` so the host can mount it: `ENTRYPOINT ["<test-runner>", "--coverage-output=/out/coverage.out"]`
- Never ship this stage to production

### Stage 3 — `artifact`
The distributable. As small as possible.

- Use a minimal base (`alpine`, `distroless`, `scratch`) — pinned version
- Strip the package manager if using alpine (removes `apk` attack surface)
- Copy **only** the built binary/assets from `base` with `COPY --from=base`
- Create a system group and non-root user
- `chown` the app directory and `/tmp` to that user
- `USER nonroot` before `ENTRYPOINT`
- `EXPOSE` the app port

## Layer Ordering Rules

```
# GOOD — dependency layer cached separately from source
COPY package.json package-lock.json ./
RUN npm ci --omit=dev
COPY ./src ./src
RUN npm run build

# BAD — source change invalidates dependency install
COPY . .
RUN npm ci
```

## Non-root User Pattern (Alpine)

```dockerfile
RUN addgroup --system nonroot && \
    adduser --system --ingroup nonroot nonroot
ENV HOME=/home/nonroot
RUN chown -R nonroot:nonroot $HOME && \
    chown -R nonroot:nonroot /tmp
USER nonroot
WORKDIR /home/nonroot
```

## Alpine Package Manager Removal (artifact stage)

```dockerfile
RUN rm -f /sbin/apk && \
    rm -rf /etc/apk /lib/apk /usr/share/apk /var/lib/apk
```

## Language Quick-Reference

See [REFERENCE.md](REFERENCE.md) for full examples per language.

| Language | Builder image | Artifact base | Test runner |
|---|---|---|---|
| Go | `golang:<version>-alpine` | `alpine:<version>` | `gotestsum` |
| Node.js | `node:<version>-alpine` | `node:<version>-alpine` or `alpine` | `jest --coverage` |
| Python | `python:<version>-slim` | `python:<version>-slim` | `pytest --cov` |
| Java | `eclipse-temurin:<version>` | `eclipse-temurin:<version>-jre` | `mvn test` / `gradle test` |
| Rust | `rust:<version>-alpine` | `alpine:<version>` or `scratch` | `cargo test` |

## Checklist

Before finalising any Dockerfile:

- [ ] All `FROM` lines use pinned, specific versions (no `latest`)
- [ ] Three stages present: `base`, `test`, `artifact`
- [ ] Dependency files copied and installed before source code
- [ ] `test` stage outputs coverage to `/out/`
- [ ] `artifact` stage uses a minimal base image
- [ ] `apk` removed from alpine artifact stage (if applicable)
- [ ] Non-root user created and active in artifact stage
- [ ] `EXPOSE` declares the application port
- [ ] No secrets, credentials, or `.env` files copied into any layer
