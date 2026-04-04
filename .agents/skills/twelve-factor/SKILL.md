---
name: twelve-factor
description: Guide for 12-Factor cloud-native applications. Use when designing microservices, configuring containers, deploying to Kubernetes, or following cloud-native patterns.
license: MIT
---

# 12-Factor App Methodology

Guide for building scalable, maintainable, and portable cloud-native applications following the 12-Factor App principles and modern extensions.

## When to Activate

Use this skill when:
- Designing or refactoring cloud-native applications
- Building applications for Kubernetes deployment
- Setting up CI/CD pipelines
- Implementing microservices architecture
- Migrating applications to containers
- Reviewing architecture for cloud readiness
- Troubleshooting deployment or scaling issues
- Working with environment configuration

## The 12 Factors

### I. Codebase
One codebase tracked in revision control, many deploys. Single Git repo; environment-specific config separate from code.

**Anti-patterns:** ❌ Multiple repos for one app ❌ Different codebases per environment

### II. Dependencies
Explicitly declare and isolate all dependencies using package managers and lock files. Never rely on system-wide packages. Use multi-stage Docker builds for isolation.

### III. Config
Store all config in environment variables — never hardcoded, never committed to version control. Use Kubernetes ConfigMaps for non-sensitive config, Secrets for sensitive values.

**Anti-patterns:** ❌ Hardcoded config ❌ Config files in version control ❌ Environment-specific code paths

### IV. Backing Services
Treat databases, queues, caches, and APIs as attached resources connected via URL in environment variables. Swappable without code changes.

### V. Build, Release, Run
Strictly separate stages: **Build** (compile) → **Release** (build + config) → **Run** (execute). Immutable releases with unique IDs (git SHA). Rollback = redeploy previous release.

### VI. Processes
Stateless, share-nothing processes. All persistent state in backing services. No sticky sessions, no local filesystem for state. Enables horizontal scaling.

### VII. Port Binding
Self-contained apps bind to a port (`0.0.0.0`, port from env var). No runtime injection (Apache/Nginx as process manager).

### VIII. Concurrency
Scale out via process model (horizontal), not vertical scaling. Different process types (web, worker, scheduler) scale independently.

### IX. Disposability
Fast startup (< 10s), graceful SIGTERM shutdown (finish in-flight requests, close connections). Robust against sudden death.

### X. Dev/Prod Parity
Keep all environments as similar as possible. Use containers (Docker Compose locally), same backing service types, same deployment process.

### XI. Logs
Treat logs as event streams — write unbuffered to stdout/stderr. Use structured JSON logging. Let the platform route logs (Fluentd, Logstash). Never manage log files in the app.

**Anti-patterns:** ❌ Writing to log files ❌ Log rotation in app ❌ Sending logs directly to aggregation service

### XII. Admin Processes
Run admin tasks (migrations, one-off scripts) as one-off processes using same codebase and config. Use Kubernetes Jobs for migrations, CronJobs for scheduled tasks.

## Modern Extensions

- **XIII. API First** — OpenAPI specs, contract-first development, API versioning
- **XIV. Telemetry** — `/metrics` endpoint (Prometheus), distributed tracing (OpenTelemetry), health checks
- **XV. Security** — OAuth 2.0/OIDC, RBAC, TLS everywhere, secrets in env, security scanning in CI/CD

## Common Patterns

### Configuration Validation at Startup
```javascript
const required = ['DATABASE_URL', 'JWT_SECRET', 'REDIS_URL'];
const missing = required.filter(key => !process.env[key]);
if (missing.length > 0) throw new Error(`Missing env vars: ${missing.join(', ')}`);
```

### Health + Readiness Endpoints
```javascript
app.get('/health', (req, res) => res.json({ status: 'healthy' }));
app.get('/ready', async (req, res) => {
  await db.ping(); await redis.ping();
  res.json({ status: 'ready' });
});
```

### Graceful Shutdown
```javascript
process.on('SIGTERM', () => {
  server.close(() => { db.close(); redis.quit(); process.exit(0); });
});
```

## Key Anti-Patterns to Avoid

- ❌ Environment-specific code paths (`if NODE_ENV === 'production'`)
- ❌ Local file storage for uploads (use object storage)
- ❌ In-memory session/state (use Redis)
- ❌ Hardcoded service locations (use env vars)

## Resources

- **12factor.net**: Original methodology
- **Kubernetes Docs**: https://kubernetes.io/docs/
- **OpenTelemetry**: https://opentelemetry.io/
